//! Cloudflare KV REST transport for `blog publish --local` — the break-glass
//! publish path (ADR-0007). Thin shim per the PRD testing decisions: all
//! decisions live in `publish-core`; this just moves the plan over HTTPS.

use content_ast::IndexEntry;
use publish_core::{PublishPlan, INDEX_KEY};

const SETUP_HINT: &str = "see docs/guides/publishing.md for the scoped-token setup";

pub struct KvClient {
    /// `…/accounts/{account}/storage/kv/namespaces/{namespace}`.
    base: String,
    token: String,
}

impl KvClient {
    pub fn from_env() -> Result<Self, String> {
        let var = |name: &str| {
            std::env::var(name).map_err(|_| format!("missing env var {name}; {SETUP_HINT}"))
        };
        let account = var("CLOUDFLARE_ACCOUNT_ID")?;
        let namespace = var("BLOG_KV_NAMESPACE_ID")?;
        let token = var("CLOUDFLARE_API_TOKEN")?;
        Ok(Self {
            base: format!(
                "https://api.cloudflare.com/client/v4/accounts/{account}/storage/kv/namespaces/{namespace}"
            ),
            token,
        })
    }

    /// Reads the current `index`; a missing key is an empty index (nothing
    /// published yet), any other failure aborts before writes happen.
    pub fn read_index(&self) -> Result<Vec<IndexEntry>, String> {
        let mut response = ureq::get(format!("{}/values/{INDEX_KEY}", self.base))
            .header("Authorization", self.bearer())
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|err| format!("GET {INDEX_KEY}: {err}"))?;
        match response.status().as_u16() {
            404 => Ok(Vec::new()),
            200 => response
                .body_mut()
                .read_json()
                .map_err(|err| format!("stored {INDEX_KEY} is not valid index JSON: {err}")),
            status => Err(format!("GET {INDEX_KEY}: HTTP {status}")),
        }
    }

    /// Applies the plan: bulk-write posts + index, then bulk-delete removals.
    pub fn apply(&self, plan: &PublishPlan) -> Result<(), String> {
        let pairs: Vec<_> = plan
            .writes
            .iter()
            .map(|w| serde_json::json!({ "key": w.key, "value": w.value }))
            .collect();
        self.send("PUT bulk", ureq::put(format!("{}/bulk", self.base)), &pairs)?;
        if !plan.deletes.is_empty() {
            self.send(
                "POST bulk/delete",
                ureq::post(format!("{}/bulk/delete", self.base)),
                &plan.deletes,
            )?;
        }
        Ok(())
    }

    fn bearer(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn send<T: serde::Serialize>(
        &self,
        what: &str,
        request: ureq::RequestBuilder<ureq::typestate::WithBody>,
        body: &T,
    ) -> Result<(), String> {
        let outcome: serde_json::Value = request
            .header("Authorization", self.bearer())
            .send_json(body)
            .map_err(|err| format!("{what}: {err}"))?
            .body_mut()
            .read_json()
            .map_err(|err| format!("{what}: unreadable response: {err}"))?;
        // 200 with success=false happens on partial bulk failures.
        if outcome["success"] == serde_json::Value::Bool(true) {
            Ok(())
        } else {
            Err(format!("{what}: Cloudflare reported failure: {outcome}"))
        }
    }
}

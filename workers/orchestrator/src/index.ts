import { getWebhooks } from './webhooks';
import { type PostChange, type PushEvent, extractPostName, validatePushEvent } from '@repo/schemas';
import BundlerWorkerEntryPoint from '../../bundler/src';

export interface Env {
	GITHUB_WEBHOOK_SECRET: string;
	BUNDLER: Service<typeof BundlerWorkerEntryPoint>;
}

export default {
	async fetch(request, env, ctx): Promise<Response> {
		const rawBody = await request.clone().text();
		const webhooks = getWebhooks(env.GITHUB_WEBHOOK_SECRET);

		const signature = request.headers.get('x-hub-signature-256');
		if (!signature) {
			return new Response('Unauthorized', { status: 401 });
		}

		if (!(await webhooks?.verify(rawBody, signature))) {
			return new Response('Unauthorized', { status: 401 });
		}

		let body: PushEvent;

		try {
			body = validatePushEvent(await request.clone().json());
		} catch (error) {
			return new Response('Invalid push event payload', { status: 400 });
		}

		if (body.ref !== 'refs/heads/main') {
			return new Response('No changes to `main` branch', { status: 204 });
		}

		const postNames = new Set<string>();
		let hasCalledBuilderWorker = false;

		const allPaths = [...(body.head_commit?.added ?? []), ...(body.head_commit?.modified ?? []), ...(body.head_commit?.removed ?? [])];
		allPaths.forEach((path) => {
			const postName = extractPostName(path);
			if (postName) {
				postNames.add(postName);
			} else if (!hasCalledBuilderWorker) {
				// ctx.waitUntil(env.FRONTEND_BUILDER.fetch(request);
				hasCalledBuilderWorker = true;
			}
		});

		const shouldCallBundlerWorker = postNames.size;
		if (shouldCallBundlerWorker) {
			const postChanges: PostChange[] = Array.from(postNames).map((postName) => ({
				postName,
				ownerName: body.repository.owner.name,
				repoName: body.repository.name,
			}));

			ctx.waitUntil(env.BUNDLER.bundlePosts(postChanges, 'chrishiguto', 'chris'));
		}

		return new Response(
			JSON.stringify({
				bundler: shouldCallBundlerWorker,
				builder: hasCalledBuilderWorker,
			}),
			{ status: 200 },
		);
	},
} satisfies ExportedHandler<Env>;

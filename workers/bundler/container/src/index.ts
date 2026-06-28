import { Hono } from "hono";
import { bundleMDX } from "mdx-bundler";

const app = new Hono();

app.post("/", async (c) => {
  const {
    payload: { source, files },
  } = await c.req.json();

  if (!source) {
    return new Response("Failed");
  }

  const result = await bundleMDX({
    source,
    files,
  });

  console.log("Post has been bundled successfully.");
  return c.json(result);
});

app.post("/bundle/bulk", async (c) => {
  const { payload } = await c.req.json();

  // Validate we received an array
  if (!Array.isArray(payload)) {
    return c.json({ error: "Payload must be an array" }, 400);
  }

  // Return empty array if no items to process
  if (payload.length === 0) {
    return c.json([]);
  }

  console.log(`Processing bulk bundle request for ${payload.length} posts`);

  const results = await Promise.all(
    payload.map(async ({ post, source, files }, index) => {
      try {
        if (!source) {
          throw new Error("Source cannot be empty");
        }
        console.log("source", source);
        console.log("files", files);

        const result = await bundleMDX({
          source,
          files,
        });

        console.log(`Post ${index + 1}/${payload.length} bundled successfully`);
        return {
          postName: post.postName,
          ...result,
        };
      } catch (error) {
        console.error(`Error bundling post ${index + 1}:`, error);
        throw error;
      }
    }),
  );

  console.log(`Bulk bundle complete: ${results.length} posts processed`);
  return c.json(results);
});

export default {
  port: 8080,
  fetch: app.fetch,
};

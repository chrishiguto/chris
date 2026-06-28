import { notFound } from "@tanstack/react-router";
import { env } from "cloudflare:workers";
import { createServerFn } from "@tanstack/react-start";
import { Post } from "../../../../../packages/schemas/src/post";

export const fetchPost = createServerFn({ method: "POST" })
  .inputValidator((d: string) => d)
  .handler(async ({ data }) => {
    console.info(`Fetching post with id ${data}...`);

    const post = await env.RESOLVER.getPost(data);
    console.log("pOST???", post);

    if (!post) {
      throw notFound();
    }

    return post as Post;
  });

export const fetchPosts = createServerFn().handler(async () => {
  console.info("Fetching posts...");
  const posts = await env.RESOLVER.getPosts();

  // TODO: validate using zod
  if (!posts) {
    throw new Error("Failed to fetch posts");
  }

  return posts;
});

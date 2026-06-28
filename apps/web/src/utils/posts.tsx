import { notFound } from "@tanstack/react-router";
import { env } from "cloudflare:workers";
import { createServerFn } from "@tanstack/react-start";

export type PostType = {
  id: number;
  title: string;
  body: string;
};

export const fetchPost = createServerFn({ method: "POST" })
  .inputValidator((d: string) => d)
  .handler(async ({ data }) => {
    console.info(`Fetching post with id ${data}...`);
    const res = await fetch(
      `https://jsonplaceholder.typicode.com/posts/${data}`,
    );
    if (!res.ok) {
      if (res.status === 404) {
        throw notFound();
      }

      throw new Error("Failed to fetch post");
    }

    const post = await res.json();

    return post as PostType;
  });

export const fetchPosts = createServerFn().handler(async () => {
  // Try to get the cached value  in KV.
  // if the value doesnt exist in KV -> dispatch worker
  // if it exists just grab it
  const list = await env.POSTS_CACHE.list();
  console.log("list of keys: ", list);
  const post2 = await env.POSTS_CACHE.get("mdx:post:hello-world");
  console.log("Post from server function: ", post2);
  console.info("Fetching posts...");
  const res = await fetch("https://jsonplaceholder.typicode.com/posts");
  if (!res.ok) {
    throw new Error("Failed to fetch posts");
  }

  const posts = await res.json();

  return (posts as Array<PostType>).slice(0, 10);
});

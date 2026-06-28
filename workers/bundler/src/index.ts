import { Container, getRandom } from "@cloudflare/containers";
import { getOctokit } from "./octokit";
import {
  validateBulkBundlerResponse,
  type PostSummary,
  type PostChange,
} from "@repo/schemas";
import { getAllBlogPostSlugs } from "./github";
import { getPostsMdxContent, PostMdxContent } from "./bundler";
import { WorkerEntrypoint } from "cloudflare:workers";
import { Post } from "../../../packages/schemas/src/post";

export interface Env {
  GITHUB_TOKEN: string;
  POSTS_CACHE: KVNamespace;
  BUNDLER_CONTAINER: DurableObjectNamespace<BundlerContainer>;
  GITHUB_OWNER?: string;
  GITHUB_REPO?: string;
}

export class BundlerContainer extends Container<Env> {
  defaultPort = 8080;
  sleepAfter = "1m";
}

export default class extends WorkerEntrypoint<Env> {
  /**
   * Bundle specific posts
   * @param posts - Array of post changes to process
   * @param owner - GitHub repository owner
   * @param repo - GitHub repository name
   * @returns Processing summary
   */
  async bundlePosts(
    posts: PostChange[],
    owner: string,
    repo: string,
  ): Promise<PostSummary[]> {
    if (!posts.length) {
      return [];
    }

    const client = getOctokit(this.env.GITHUB_TOKEN);

    if (!client) {
      throw new Error("Client has failed to spawn");
    }

    const postsFiles = await getPostsMdxContent(client, posts);
    const bundledPosts = validateBulkBundlerResponse(
      await this._getBundledPosts(postsFiles),
    );

    await Promise.allSettled(
      bundledPosts.map(({ postName, frontmatter, code }) =>
        this.env.POSTS_CACHE.put(
          `mdx:post:${postName}`,
          JSON.stringify({
            id: postName,
            postName,
            title: frontmatter.title,
            date: frontmatter.date,
            code,
          }),
        ),
      ),
    );

    const allPostNames = await getAllBlogPostSlugs(client, owner, repo);
    const allPostsCache: PostSummary[] = [];

    for (const { postName } of allPostNames) {
      // TODO: validate with zod instead of typing the get
      const post = await this.env.POSTS_CACHE.get<Post>(
        `mdx:post:${postName}`,
        "json",
      );
      if (post) {
        allPostsCache.push({
          id: post.id,
          postName: post.postName,
          title: post.title,
          date: post.date,
        });
      }
    }

    await this.env.POSTS_CACHE.put(
      `mdx:post:all`,
      JSON.stringify(allPostsCache),
    );

    return allPostsCache;
  }

  /**
   * Bundle all posts (manual refresh / cache miss)
   * @param owner - GitHub repository owner
   * @param repo - GitHub repository name
   * @returns Processing summary
   */
  async bundleAllPosts(owner: string, repo: string): Promise<PostSummary[]> {
    const client = getOctokit(this.env.GITHUB_TOKEN);

    if (!client) {
      throw new Error("Failed to initialize GitHub client");
    }

    const postsPath = await getAllBlogPostSlugs(client, owner, repo);
    const postsFiles = await getPostsMdxContent(client, postsPath);
    const bundledPosts = validateBulkBundlerResponse(
      await this._getBundledPosts(postsFiles),
    );

    const allPostsCache = bundledPosts.map(({ postName, frontmatter }) => ({
      id: postName,
      postName,
      title: frontmatter.title,
      date: frontmatter.date,
    }));

    await Promise.allSettled([
      ...bundledPosts.map(({ postName, frontmatter, code }) =>
        this.env.POSTS_CACHE.put(
          `mdx:post:${postName}`,
          JSON.stringify({
            id: postName,
            postName,
            title: frontmatter.title,
            date: frontmatter.date,
            code,
          }),
        ),
      ),
      this.env.POSTS_CACHE.put(`mdx:post:all`, JSON.stringify(allPostsCache)),
    ]);

    console.log(`✓ Cached ${bundledPosts.length} bundled posts`);
    return allPostsCache;
  }

  async _getBundledPosts(postFiles: PostMdxContent[] | []) {
    if (postFiles.length === 0) {
      console.log("No posts to bundle");
      return [];
    }

    try {
      const container = await getRandom(this.env.BUNDLER_CONTAINER, 3);
      const result = await container.fetch("http://container.com/bundle/bulk", {
        method: "POST",
        body: JSON.stringify({ payload: postFiles }),
      });

      return await result.json();
    } catch (error) {
      console.error("Error during bulk bundling:", error);
      throw error;
    }
  }
}

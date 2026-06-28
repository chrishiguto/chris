import type { Octokit } from "@octokit/rest";
import type { PostChange, BundlerPayload } from "@repo/schemas";
import { collectPostFiles } from "./github";

/**
 * Data structure to track posts for bulk bundling
 */
export interface PostMdxContent {
  post: PostChange;
  source: string;
  files: Record<string, string>;
}

/**
 * Shared bundling logic used by both bundlePosts() and bundleAllPosts()
 * Processes an array of PostChange objects through the bundling pipeline
 * @param client - Octokit client instance
 * @param posts - Array of post changes to process
 * @returns Processing summary with counts and metadata
 */
export async function getPostsMdxContent(
  client: Octokit,
  posts: PostChange[],
): Promise<PostMdxContent[] | []> {
  const postsToBundle: PostMdxContent[] = [];

  console.log(`Collecting files for ${posts.length} posts`);

  for (const post of posts) {
    const folderPath = `content/blog/${post.postName}`;

    try {
      const { source, files } = await collectPostFiles(
        client,
        post,
        folderPath,
      );

      if (!source && Object.keys(files).length === 0) {
        console.log(`Post folder empty or invalid: ${post.postName}`);
        continue;
      }

      if (!source) {
        console.warn(
          `Warning: No index.mdx found for post ${post.postName}, ignoring post`,
        );
        continue;
      }

      const dependencyCount = Object.keys(files).length;
      console.log(
        `✓ Successfully collected source and ${dependencyCount} ${dependencyCount === 1 ? "dependency" : "dependencies"} for ${post.postName}`,
      );

      postsToBundle.push({ post, source, files });
    } catch (error: any) {
      console.error(`Error fetching post ${post.postName}:`, error);
    }
  }

  return postsToBundle;
}

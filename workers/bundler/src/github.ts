import type { Octokit } from "@octokit/rest";
import type { PostChange } from "@repo/schemas";

/**
 * Result of collecting post files from GitHub
 */
export interface PostFiles {
  source: string | undefined;
  files: Record<string, string>;
}

/**
 * Fetches and decodes a blob from GitHub
 * @param client - Octokit instance
 * @param owner - Repository owner
 * @param repo - Repository name
 * @param sha - Blob SHA
 * @returns Decoded blob content
 */
export async function fetchBlobContent(
  client: Octokit,
  owner: string,
  repo: string,
  sha: string,
): Promise<string> {
  const {
    data: { content },
  } = await client.git.getBlob({
    owner,
    repo,
    file_sha: sha,
  });

  return atob(content);
}

/**
 * Recursively collects all files from a GitHub directory
 * @param client - Octokit instance
 * @param post - Post change information
 * @param currentPath - Current directory path being processed
 * @param baseFolderPath - Base folder path (e.g., "content/blog/hello-world")
 * @param result - Accumulated result object (mutated)
 */
async function collectFilesRecursive(
  client: Octokit,
  post: PostChange,
  currentPath: string,
  baseFolderPath: string,
  result: PostFiles,
): Promise<void> {
  const { data } = await client.repos.getContent({
    owner: post.ownerName,
    repo: post.repoName,
    path: currentPath,
  });

  if (!Array.isArray(data)) {
    // Single file returned, shouldn't happen for directories
    return;
  }

  for (const item of data) {
    if (item.type === "file") {
      // Fetch and decode file content
      const decodedContent = await fetchBlobContent(
        client,
        post.ownerName,
        post.repoName,
        item.sha!,
      );

      // Calculate relative path from base folder
      const filename = item.path.replace(`${baseFolderPath}/`, "");

      if (filename === "index.mdx") {
        result.source = decodedContent;
        console.log(`  - Found source: ${filename}`);
      } else if (filename.endsWith(".tsx")) {
        const relativePath = `./${filename}`;
        result.files[relativePath] = decodedContent;
        console.log(`  - Found dependency: ${relativePath}`);
      }
    } else if (item.type === "dir") {
      await collectFilesRecursive(
        client,
        post,
        item.path,
        baseFolderPath,
        result,
      );
    }
  }
}

/**
 * Collects all MDX source and dependency files for a blog post
 * @param client - Octokit instance
 * @param post - Post change information
 * @param folderPath - Post folder path (e.g., "content/blog/hello-world")
 * @returns Object containing source and dependency files
 */
export async function collectPostFiles(
  client: Octokit,
  post: PostChange,
  folderPath: string,
): Promise<PostFiles> {
  const result: PostFiles = {
    source: undefined,
    files: {},
  };

  await collectFilesRecursive(client, post, folderPath, folderPath, result);

  return result;
}

/**
 * Fetches all blog post folder names from the GitHub repository
 * Uses repos.getContent API to list folders in content/blog directory
 * @param client - Octokit instance
 * @param owner - Repository owner
 * @param repo - Repository name
 * @returns Array of post folder names (e.g., ["hello-world", "my-post"])
 */
export async function getAllBlogPostSlugs(
  client: Octokit,
  owner: string,
  repo: string,
): Promise<PostChange[]> {
  const { data } = await client.repos.getContent({
    owner,
    repo,
    path: "content/blog",
  });

  if (!Array.isArray(data)) {
    return [];
  }

  return data.map((item) => ({
    postName: item.name,
    ownerName: owner,
    repoName: repo,
  }));
}

import { z } from "zod";

/**
 * Schema for post changes from GitHub webhooks
 * Validates that:
 * - postName is the blog post folder name (e.g., "hello-world")
 * - ownerName and repoName are non-empty strings
 *
 * The compiler determines the operation (add/modify/delete) by checking
 * if the post folder exists in the GitHub repository.
 */
export const PostChangeSchema = z.object({
  postName: z.string().min(1, "Post name cannot be empty"),
  ownerName: z.string().min(1, "Owner name cannot be empty"),
  repoName: z.string().min(1, "Repo name cannot be empty"),
});

/**
 * Inferred TypeScript type for PostChange
 */
export type PostChange = z.infer<typeof PostChangeSchema>;

/**
 * Schema for an array of post changes
 */
export const PostChangesSchema = z.array(PostChangeSchema);

/**
 * Validates a single post change object
 * @param data - Unknown data to validate
 * @returns Validated PostChange object
 * @throws ZodError if validation fails
 */
export function validatePostChange(data: unknown): PostChange {
  return PostChangeSchema.parse(data);
}

/**
 * Validates an array of post changes
 * @param data - Unknown data to validate
 * @returns Validated array of PostChange objects
 * @throws ZodError if validation fails
 */
export function validatePostChanges(data: unknown): PostChange[] {
  return PostChangesSchema.parse(data);
}

/**
 * Safely validates a single post change object
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidatePostChange(data: unknown) {
  return PostChangeSchema.safeParse(data);
}

/**
 * Safely validates an array of post changes
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidatePostChanges(data: unknown) {
  return PostChangesSchema.safeParse(data);
}

/**
 * Schema for a complete blog post including compiled MDX code
 * Contains all information about a post including the bundled code
 */
export const PostSchema = z.object({
  id: z.string().min(1, "Post ID cannot be empty"),
  postName: z.string().min(1, "Post name cannot be empty"),
  title: z.string().optional(),
  date: z.string().optional(),
  // description: z.string().optional(),
  // tags: z.array(z.string()).optional(),
  code: z.string(),
});

/**
 * Inferred TypeScript type for Post
 */
export type Post = z.infer<typeof PostSchema>;

/**
 * Schema for post summary returned by getPosts()
 * Contains basic information about a blog post for listing purposes (without compiled code)
 */
export const PostSummarySchema = PostSchema.omit({ code: true });

/**
 * Inferred TypeScript type for PostSummary
 */
export type PostSummary = z.infer<typeof PostSummarySchema>;

/**
 * Schema for an array of post summaries
 * Used by getPosts() to validate the list of all posts
 */
export const PostsListSchema = z.array(PostSummarySchema);

/**
 * Validates a single post object
 * @param data - Unknown data to validate
 * @returns Validated Post object
 * @throws ZodError if validation fails
 */
export function validatePost(data: unknown): Post {
  return PostSchema.parse(data);
}

/**
 * Safely validates a single post object
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidatePost(data: unknown) {
  return PostSchema.safeParse(data);
}

/**
 * Validates a single post summary object
 * @param data - Unknown data to validate
 * @returns Validated PostSummary object
 * @throws ZodError if validation fails
 */
export function validatePostSummary(data: unknown): PostSummary {
  return PostSummarySchema.parse(data);
}

/**
 * Safely validates a single post summary object
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidatePostSummary(data: unknown) {
  return PostSummarySchema.safeParse(data);
}

/**
 * Validates an array of post summaries
 * @param data - Unknown data to validate
 * @returns Validated array of PostSummary objects
 * @throws ZodError if validation fails
 */
export function validatePostsList(data: unknown): PostSummary[] {
  return PostsListSchema.parse(data);
}

/**
 * Safely validates an array of post summaries
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidatePostsList(data: unknown) {
  return PostsListSchema.safeParse(data);
}

/**
 * Extracts the post name from a file path
 * @param path - File path like "content/blog/hello-world/index.mdx"
 * @returns Post name like "hello-world" or null if invalid path
 * @example
 * extractPostName("content/blog/hello-world/index.mdx") // "hello-world"
 * extractPostName("content/blog/my-post/components/foo.tsx") // "my-post"
 */
export function extractPostName(path: string): string | null {
  if (!path.startsWith("content/blog/")) {
    return null;
  }

  const pathAfterBlog = path.substring("content/blog/".length);
  const postName = pathAfterBlog.split("/")[0];

  return postName || null;
}

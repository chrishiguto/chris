import { z } from "zod";
import { PostChangeSchema } from "./post";

/**
 * Schema for esbuild Location type
 * Represents the location of an error or warning in a file
 */
export const LocationSchema = z.object({
  file: z.string(),
  namespace: z.string(),
  line: z.number(), // 1-based
  column: z.number(), // 0-based, in bytes
  length: z.number(), // in bytes
  lineText: z.string(),
});

/**
 * Inferred TypeScript type for Location
 */
export type Location = z.infer<typeof LocationSchema>;

/**
 * Schema for esbuild Message type
 * Represents an error or warning message from esbuild
 */
export const EsbuildMessageSchema = z.object({
  text: z.string(),
  location: LocationSchema.nullable(),
  detail: z.any().optional(),
});

/**
 * Inferred TypeScript type for EsbuildMessage
 */
export type EsbuildMessage = z.infer<typeof EsbuildMessageSchema>;

/**
 * Schema for MDX frontmatter
 * Flexible record to accommodate any frontmatter fields
 */
export const FrontmatterSchema = z.record(z.any());

/**
 * Inferred TypeScript type for Frontmatter
 */
export type Frontmatter = z.infer<typeof FrontmatterSchema>;

/**
 * Schema for gray-matter result object
 * Contains the parsed frontmatter data
 */
export const MatterSchema = z.object({
  data: FrontmatterSchema,
});

/**
 * Inferred TypeScript type for Matter
 */
export type Matter = z.infer<typeof MatterSchema>;

/**
 * Schema for bundler payload containing MDX source and dependencies
 * Used by the container to receive compilation work
 */
export const BundlerPayloadSchema = z.object({
  post: PostChangeSchema,
  source: z.string().min(1, "Source cannot be empty"),
  files: z.record(z.string(), z.string()),
});

/**
 * Inferred TypeScript type for BundlerPayload
 */
export type BundlerPayload = z.infer<typeof BundlerPayloadSchema>;

/**
 * Schema for bundler request wrapping the payload
 * Validates the complete request structure sent to the container
 */
export const BundlerRequestSchema = z.object({
  payload: BundlerPayloadSchema,
});

/**
 * Inferred TypeScript type for BundlerRequest
 */
export type BundlerRequest = z.infer<typeof BundlerRequestSchema>;

/**
 * Schema for the bundleMDX response from the bundler container
 * Validates the complete response including code, frontmatter, errors, and matter
 */
export const BundlerResponseSchema = z.object({
  postName: z.string(),
  code: z.string(),
  frontmatter: FrontmatterSchema,
  errors: z.array(EsbuildMessageSchema).optional().default([]),
  matter: MatterSchema,
});

/**
 * Inferred TypeScript type for BundlerResponse
 */
export type BundlerResponse = z.infer<typeof BundlerResponseSchema>;

/**
 * Validates a bundler response object
 * @param data - Unknown data to validate
 * @returns Validated BundlerResponse object
 * @throws ZodError if validation fails
 */
export function validateBundlerResponse(data: unknown): BundlerResponse {
  return BundlerResponseSchema.parse(data);
}

/**
 * Safely validates a bundler response object
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidateBundlerResponse(data: unknown) {
  return BundlerResponseSchema.safeParse(data);
}

/**
 * Schema for bulk bundler payload containing multiple MDX sources
 * Used by the container to receive multiple compilation tasks
 */
export const BulkBundlerPayloadSchema = z.array(BundlerPayloadSchema);

/**
 * Inferred TypeScript type for BulkBundlerPayload
 */
export type BulkBundlerPayload = z.infer<typeof BulkBundlerPayloadSchema>;

/**
 * Schema for bulk bundler request wrapping the payload array
 * Validates the complete request structure sent to the container
 */
export const BulkBundlerRequestSchema = z.object({
  payload: BulkBundlerPayloadSchema,
});

/**
 * Inferred TypeScript type for BulkBundlerRequest
 */
export type BulkBundlerRequest = z.infer<typeof BulkBundlerRequestSchema>;

/**
 * Schema for bulk bundler response containing multiple bundled results
 * Validates the complete response array from the container
 */
export const BulkBundlerResponseSchema = z.array(BundlerResponseSchema);

/**
 * Inferred TypeScript type for BulkBundlerResponse
 */
export type BulkBundlerResponse = z.infer<typeof BulkBundlerResponseSchema>;

/**
 * Validates a bulk bundler request object
 * @param data - Unknown data to validate
 * @returns Validated BulkBundlerRequest object
 * @throws ZodError if validation fails
 */
export function validateBulkBundlerRequest(data: unknown): BulkBundlerRequest {
  return BulkBundlerRequestSchema.parse(data);
}

/**
 * Validates a bulk bundler response object
 * @param data - Unknown data to validate
 * @returns Validated BulkBundlerResponse object
 * @throws ZodError if validation fails
 */
export function validateBulkBundlerResponse(
  data: unknown,
): BulkBundlerResponse {
  return BulkBundlerResponseSchema.parse(data);
}

/**
 * Safely validates a bulk bundler response object
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidateBulkBundlerResponse(data: unknown) {
  return BulkBundlerResponseSchema.safeParse(data);
}

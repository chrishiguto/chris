import { z } from "zod";

/**
 * Schema for GitHub webhook push events
 * Validates the subset of PushEvent properties used by the orchestrator worker
 *
 * Essential properties:
 * - ref: Git reference (e.g., "refs/heads/main")
 * - head_commit: Contains file change arrays (added, modified, removed)
 * - repository: Repository metadata (owner, name)
 *
 * Additional metadata included for potential future use:
 * - before/after: Commit SHAs
 * - created/deleted/forced: Push metadata flags
 */
export const PushEventSchema = z.object({
  ref: z.string(),
  before: z.string().optional(),
  after: z.string().optional(),
  created: z.boolean().optional(),
  deleted: z.boolean().optional(),
  forced: z.boolean().optional(),
  head_commit: z
    .object({
      added: z.array(z.string()).optional(),
      modified: z.array(z.string()).optional(),
      removed: z.array(z.string()).optional(),
    })
    .nullable()
    .optional(),
  repository: z.object({
    name: z.string(),
    owner: z.object({
      name: z.string(),
    }),
  }),
});

/**
 * Inferred TypeScript type for PushEvent
 */
export type PushEvent = z.infer<typeof PushEventSchema>;

/**
 * Validates a GitHub push event payload
 * @param data - Unknown data to validate
 * @returns Validated PushEvent object
 * @throws ZodError if validation fails
 */
export function validatePushEvent(data: unknown): PushEvent {
  return PushEventSchema.parse(data);
}

/**
 * Safely validates a GitHub push event payload
 * @param data - Unknown data to validate
 * @returns Success result with data or error result
 */
export function safeValidatePushEvent(data: unknown) {
  return PushEventSchema.safeParse(data);
}

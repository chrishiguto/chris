import { Webhooks } from '@octokit/webhooks';

const webhooksCache = new Map<string, Webhooks>();

export function getWebhooks(secret: string): Webhooks | undefined {
	if (!webhooksCache.has(secret)) {
		webhooksCache.set(secret, new Webhooks({ secret }));
	}
	return webhooksCache.get(secret);
}

import { Octokit } from '@octokit/rest';

const octokitCache = new Map<string, Octokit>();

export function getOctokit(auth: string): Octokit | undefined {
	if (!octokitCache.has(auth)) {
		octokitCache.set(auth, new Octokit({ auth }));
	}
	return octokitCache.get(auth);
}

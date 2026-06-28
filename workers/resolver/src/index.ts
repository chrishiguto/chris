import { WorkerEntrypoint } from 'cloudflare:workers';
import { type PostSummary, validatePostsList } from '@repo/schemas';
import BundlerWorkerEntryPoint from '../../bundler/src';
import { validatePost } from '../../../packages/schemas/src/post';

export interface Env {
	POSTS_CACHE: KVNamespace;
	BUNDLER: Service<typeof BundlerWorkerEntryPoint>;
	GITHUB_OWNER: string;
	GITHUB_REPO: string;
}

export default class extends WorkerEntrypoint<Env> {
	async getPosts(): Promise<PostSummary[]> {
		const cached = await this.env.POSTS_CACHE.get('mdx:post:all', 'json');
		console.log('Cached posts?', cached);

		if (cached) {
			return validatePostsList(cached);
		}

		try {
			const posts = await this.env.BUNDLER.bundleAllPosts('chrishiguto', 'chris');

			if (posts) {
				return validatePostsList(posts);
			}

			console.warn('No posts found after bundling');
			return [];
		} catch (error) {
			console.error('Error calling bundler:', error);
			return [];
		}
	}

	async getPost(id: string) {
		const cached = await this.env.POSTS_CACHE.get(`mdx:post:${id}`, 'json');

		if (cached) {
			return validatePost(cached);
		}

		return null;
	}
}

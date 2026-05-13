import { describe, expect, it } from 'vitest';

import { insertOptimistic, reconcile } from './optimistic';
import type { SessionTurn } from '$lib/studio';

describe('optimistic timeline store', () => {
	it('inserts and reconciles a user turn', () => {
		const optimistic = insertOptimistic([], {
			kind: 'user_intent',
			id: 'local',
			at: '1',
			raw: 'build',
			source_kind: 'text'
		});
		const server: SessionTurn = {
			kind: 'user_intent',
			id: 'server',
			at: '2',
			raw: 'build',
			source_kind: 'text'
		};

		const reconciled = reconcile(optimistic, server);

		expect(reconciled).toHaveLength(1);
		expect(reconciled[0].id).toBe('server');
	});
});

import type { SessionTurn } from '$lib/studio';

export type OptimisticTurn = SessionTurn & { optimistic?: boolean };

export function insertOptimistic(turns: OptimisticTurn[], turn: OptimisticTurn): OptimisticTurn[] {
	return [...turns, { ...turn, optimistic: true }];
}

export function reconcile(turns: OptimisticTurn[], serverTurn: SessionTurn): OptimisticTurn[] {
	const withoutMatching = turns.filter((turn) => !(turn.optimistic && turn.kind === serverTurn.kind));
	if (withoutMatching.some((turn) => turn.id === serverTurn.id)) return withoutMatching;
	return [...withoutMatching, serverTurn];
}

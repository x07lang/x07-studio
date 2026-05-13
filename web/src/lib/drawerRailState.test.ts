import { describe, expect, it } from 'vitest';

import { drawerRailStorageKey, loadDrawerState, saveDrawerState } from './drawerRailState';

function memoryStorage(): Storage {
	const store = new Map<string, string>();
	return {
		get length() {
			return store.size;
		},
		clear: () => store.clear(),
		getItem: (key: string) => store.get(key) ?? null,
		key: (index: number) => Array.from(store.keys())[index] ?? null,
		removeItem: (key: string) => {
			store.delete(key);
		},
		setItem: (key: string, value: string) => {
			store.set(key, value);
		}
	};
}

describe('drawer rail state', () => {
	it('loads and saves drawer openness', () => {
		const storage = memoryStorage();
		saveDrawerState({ now: true }, storage);
		expect(storage.getItem(drawerRailStorageKey)).toBe('{"now":true}');
		expect(loadDrawerState(storage)).toEqual({ now: true });
	});

	it('falls back to closed state on malformed persisted JSON', () => {
		const storage = memoryStorage();
		storage.setItem(drawerRailStorageKey, '{');
		expect(loadDrawerState(storage)).toEqual({});
	});
});

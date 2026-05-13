export type DrawerRailState = Record<string, boolean>;

export const drawerRailStorageKey = 'x07-studio.drawer-rail';

export function loadDrawerState(storage: Storage | null | undefined = globalThis.localStorage) {
	if (!storage) return {};
	try {
		return JSON.parse(storage.getItem(drawerRailStorageKey) ?? '{}') as DrawerRailState;
	} catch {
		return {};
	}
}

export function saveDrawerState(
	open: DrawerRailState,
	storage: Storage | null | undefined = globalThis.localStorage
) {
	if (!storage) return;
	storage.setItem(drawerRailStorageKey, JSON.stringify(open));
}

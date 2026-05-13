export interface StudioStorage {
	get<T>(key: string): T | null;
	set<T>(key: string, value: T): void;
	remove(key: string): void;
}

export class LocalStorageAdapter implements StudioStorage {
	get<T>(key: string): T | null {
		if (typeof localStorage === 'undefined') return null;
		const raw = localStorage.getItem(key);
		return raw ? (JSON.parse(raw) as T) : null;
	}
	set<T>(key: string, value: T): void {
		if (typeof localStorage === 'undefined') return;
		localStorage.setItem(key, JSON.stringify(value));
	}
	remove(key: string): void {
		if (typeof localStorage !== 'undefined') localStorage.removeItem(key);
	}
}

export class MemoryAdapter implements StudioStorage {
	private values = new Map<string, unknown>();
	get<T>(key: string): T | null {
		return (this.values.get(key) as T | undefined) ?? null;
	}
	set<T>(key: string, value: T): void {
		this.values.set(key, value);
	}
	remove(key: string): void {
		this.values.delete(key);
	}
}

export class WebUiStorageAdapter implements StudioStorage {
	get<T>(key: string): T | null {
		window.parent?.postMessage({ effect: 'std.web_ui.effects.storage.get', key }, '*');
		return null;
	}
	set<T>(key: string, value: T): void {
		window.parent?.postMessage({ effect: 'std.web_ui.effects.storage.set', key, value }, '*');
	}
	remove(key: string): void {
		window.parent?.postMessage({ effect: 'std.web_ui.effects.storage.remove', key }, '*');
	}
}

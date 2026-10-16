globalThis.structuredClone ??= (value: unknown) => JSON.parse(JSON.stringify(value));

export const http = {
  async get(path: string) {
    return fetch(path).then((r) => r.json());
  },
};

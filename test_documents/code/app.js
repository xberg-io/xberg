import { readFile } from "fs/promises";

export class Application {
  constructor(name) {
    this.name = name;
    this.routes = [];
  }

  addRoute(path, handler) {
    this.routes.push({ path, handler });
  }

  async start(port) {
    console.log(`${this.name} starting on port ${port}`);
  }
}

export function createApp(name) {
  return new Application(name);
}

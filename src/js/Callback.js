export default class Callback {
  static #callbacks = {};

  static set(name, callback) {
    if (this.#callbacks[name]) {
      throw new Error(`Callback ${name} already registered`);
    }
    this.#callbacks[name] = callback;
  }

  static get(name) {
    return this.#callbacks[name];
  }

  static remove(name) {
    delete this.#callbacks[name];
  }
}

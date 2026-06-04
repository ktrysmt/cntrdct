class Repository {
  find(id: number): Item {
    return this.lookup(id);
  }

  static create(): Repository {
    return new Repository();
  }
}

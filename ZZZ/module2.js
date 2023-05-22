// Base class for DefaultTestFileLoader and FakeTestFileLoader.
export class AA {
  async mess(inn) {
    return inn;
  }
}

export class Ass extends AA {
    async mama() {
        return "mama";
    }

    ma() {
        return "ma";
    }
}
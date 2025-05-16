// Property identifier in a 'method_definition' (class method)
class MyClassExample {
  classMethodRegular() {}

  static staticClassMethod() {}

  get getterMethod() {
    return this._x;
  }

  set setterMethod(value) {
    this._x = value;
  }
}

// Property identifier in a 'method_definition' (object shorthand method)
const objWithShorthand = {
  shorthandObjMethod() {},

  *generatorShorthandMethod() {},

  async asyncShorthandMethod() {},
};

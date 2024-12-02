import {Component} from 'veda-client';

export default class InputInteger extends Component(HTMLInputElement) {
  static tag = 'bpa-input-integer';

  added() {
    this.property = this.getAttribute('property');
  }

  post() {
    const getNumberValue = (model, property) => {
      const values = model[property] || [];
      const value = values[0];
      return value ? parseInt(value) : '';
    };

    this.type = 'number';
    this.step = '1';
    this.value = getNumberValue(this.model, this.property);

    this.onchange = () => {
      const numValue = this.value ? parseInt(this.value) : '';
      if (!isNaN(numValue)) {
        this.model[this.property] = [numValue];
      } else {
        this.model[this.property] = [];
      }
    };

    this.modifiedHandler = () => {
      this.value = getNumberValue(this.model, this.property);
    };

    this.model.on(this.property, this.modifiedHandler);
  }

  remove() {
    this.model.off(this.property, this.modifiedHandler);
  }
}
customElements.define(InputInteger.tag, InputInteger, {extends: 'input'});

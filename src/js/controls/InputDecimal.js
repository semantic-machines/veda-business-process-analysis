import {Component} from 'veda-client';

export default class InputDecimal extends Component(HTMLInputElement) {
  static tag = 'bpa-input-decimal';

  added() {
    this.property = this.getAttribute('property');
  }

  post() {
    const getNumberValue = (model, property) => {
      const values = model[property] || [];
      const value = values[0];
      return value ? parseFloat(value) : '';
    };

    this.type = 'number';
    this.step = 'any';
    this.value = getNumberValue(this.model, this.property);

    this.onchange = () => {
      const numValue = this.value ? parseFloat(this.value) : '';
      if (!isNaN(numValue)) {
        const value = Number.isInteger(numValue) ? numValue + '.0' : numValue;
        this.model[this.property] = [value];
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
customElements.define(InputDecimal.tag, InputDecimal, {extends: 'input'});

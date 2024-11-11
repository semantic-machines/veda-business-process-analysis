import {Component} from 'veda-client';

export default class InputText extends Component(HTMLInputElement) {
  static tag = 'bpa-input-text';

  property = this.dataset.property;

  pre() {
    const getFilteredValue = (model, property) => {
      const currentLang = document.documentElement.lang.toUpperCase();
      const regex = /\^\^[a-z]{2}$/i;
      return model[property]
        ?.filter(str => !regex.test(str) || str.toUpperCase().endsWith(`^^${currentLang}`))
        .map(str => str.split(regex)[0])
        .join(' ') ?? '';
    };

    this.value = getFilteredValue(this.model, this.property);

    this.oninput = () => {
      const existingValues = this.model[this.property] || [];
      const currentLang = document.documentElement.lang.toUpperCase();
      const regex = /\^\^[a-z]{2}$/i;
      const newValues = [...existingValues.filter(str => regex.test(str) && !str.endsWith(`^^${currentLang}`))];
      if (this.value) {
        newValues.push(`${this.value}^^${currentLang}`);
      }
      this.model[this.property] = newValues;
    };
        
    this.modifiedHandler = () => {
      const newValue = getFilteredValue(this.model, this.property);
      if (this.value !== newValue) {
        this.value = newValue;
        this.dispatchEvent(new Event('input', { bubbles: true }));
      }
    };

    this.model.on(this.property, this.modifiedHandler);
  }
  
  remove() {
    this.model.off(this.property, this.modifiedHandler);
  }
}

customElements.define(InputText.tag, InputText, {extends: 'input'});

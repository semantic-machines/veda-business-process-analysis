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

    this.onkeyup = () => {
      const existingValues = this.model[this.property] || [];
      const currentLang = document.documentElement.lang.toUpperCase();
      const regex = /\^\^[a-z]{2}$/i;
      const newValues = [
        ...existingValues.filter(str => regex.test(str) && !str.endsWith(`^^${currentLang}`)), 
        `${this.value}^^${currentLang}`
      ];
      this.model[this.property] = newValues;
    };
        
    this.modifiedHandler = () => {
      this.value = getFilteredValue(this.model, this.property);
    };

    this.model.on(this.property, this.modifiedHandler);
  }
  
  remove() {
    this.model.off(this.property, this.modifiedHandler);
  }
}

customElements.define(InputText.tag, InputText, {extends: 'input'});

import {Component, html, Model} from 'veda-client';

function getLiteralValue (model, property) {
  const currentLang = document.documentElement.lang.toUpperCase();
  const regex = /\^\^[a-z]{2}$/i;
  return model.hasValue(property) 
    ? model[property]
        ?.filter(str => !regex.test(str) || str.toString().toUpperCase().endsWith(`^^${currentLang}`))
        .map(str => str.toString().split(regex)[0])
        .join(' ') ?? ''
    : '';
}

export default class Literal extends Component(HTMLElement) {
  static tag = 'bpa-literal';

  property = this.getAttribute('property');

  render () {
    return getLiteralValue(this.model, this.property);
  }

  up = () => {
    this.update();
  }

  post () {
    this.model.on(this.property, this.up);
  }

  removed () {
    this.model.off(this.property, this.up);
  }  
}

customElements.define(Literal.tag, Literal);

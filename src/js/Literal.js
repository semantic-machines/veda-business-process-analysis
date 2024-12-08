import {Component, safe} from 'veda-client';

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

  render () {
    const maxChars = Number(this.maxChars) || Infinity;
    const value = safe(getLiteralValue(this.model, this.property));
    const truncated = value.slice(0, maxChars);
    return value.length > maxChars ? `${truncated}...` : truncated;
  }

  up = () => {
    this.update();
  }

  added () {
    this.property = this.getAttribute('property');
    this.maxChars = this.getAttribute('max-chars');
    this.model.on(this.property, this.up);
  }

  removed () {
    if (this.model && this.property) {
      this.model.off(this.property, this.up);
    }
  }
}

customElements.define(Literal.tag, Literal);

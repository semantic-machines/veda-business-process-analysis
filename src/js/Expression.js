import {Component, safe} from 'veda-client';

export default class Expression extends Component(HTMLElement) {
  static tag = 'bpa-expression';

  expression = this.getAttribute('expression');

  render () {
    return safe(new Function('return ' + this.expression).call(this.model ?? null));
  }

  up = () => {
    this.update();
  }

  added () {
    this.model?.on('modified', this.up);
  }

  removed () {
    this.model?.off('modified', this.up);
  }
}

customElements.define(Expression.tag, Expression);

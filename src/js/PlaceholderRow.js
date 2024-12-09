import {Component} from 'veda-client';

export default class PlaceholderRow extends Component(HTMLTableRowElement) {
  static tag = 'bpa-placeholder-row';

  random (min, max) {
    return Math.round(Math.random() * (max - min) + min);
  }

  evaluate () {
    return new Function('return ' + this.when).call(this);
  }

  render () {
    if (this.evaluate()) {
      this.setAttribute('disabled', '');
      return this.template.replace(/<td[\s\S]*?<\/td>/g, `
        <td class="placeholder-glow">
          ${Array(this.rows).fill().map(() => `<span class="placeholder col-${this.random(5, 12)}"></span>`).join('<br>')}
        </td>
      `);
    } else {
      this.removeAttribute('disabled');
      return this.template;
    }
  }

  up = () => {
    this.update();
  }

  added () {
    this.when = this.getAttribute('when');
    this.rows = Number(this.getAttribute('rows')) || 2;
    this.model.on('modified', this.up);
  }

  removed () {
    if (this.model) {
      this.model.off('modified', this.up);
    }
  }
}

customElements.define(PlaceholderRow.tag, PlaceholderRow, { extends: 'tr' });

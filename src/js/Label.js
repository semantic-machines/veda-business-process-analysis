import {Component, html} from 'veda-client';

export default class Label extends Component(HTMLElement) {
  static tag = 'bpa-label';

  render () {
    return this.model.hasValue('rdfs:label') ? html`<span property="rdfs:label"></span>` : html`${this.model.id}`;
  }
}

customElements.define(Label.tag, Label);

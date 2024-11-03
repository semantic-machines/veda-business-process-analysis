import {Component, html} from 'veda-client';

export default class BusinessProcessView extends Component(HTMLElement) {
  static tag = 'bpa-process-view';

  async render() {
    return html`
      <span property="rdfs:label"></span>
    `;
  }
}

customElements.define(BusinessProcessView.tag, BusinessProcessView);

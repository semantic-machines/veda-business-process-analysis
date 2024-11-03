import {Component, html} from 'veda-client';

export default class ClusterView extends Component(HTMLElement) {
  static tag = 'bpa-cluster-view';

  async render() {
    return html`
      <span property="rdfs:label"></span>
    `;
  }
}

customElements.define(ClusterView.tag, ClusterView);

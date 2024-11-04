import {Component, html} from 'veda-client';
import TTLView from './TTLView.js';

export default class ClusterView extends Component(HTMLElement) {
  static tag = 'bpa-cluster-view';
  
  render() {
    return html`
      <${TTLView} about=${this.model.id}></${TTLView}>
    `;
  }
}

customElements.define(ClusterView.tag, ClusterView);

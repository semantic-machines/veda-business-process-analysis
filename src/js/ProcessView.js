import {Component, html} from 'veda-client';
import TTLView from './TTLView.js';

export default class ProcessView extends Component(HTMLElement) {
  static tag = 'bpa-process-view';
  
  render() {
    return html`
      <${TTLView} about=${this.model.id}></${TTLView}>
    `;
  }
}

customElements.define(ProcessView.tag, ProcessView);

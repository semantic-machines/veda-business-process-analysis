import {Component, html} from 'veda-client';

export default class ProcessRelevanceIndicator extends Component(HTMLElement) {
  static tag = 'bpa-process-relevance-indicator';

  render() {
    return (
      this.model.id === 'v-bpa:CompletelyJustified'
      ? html`<span class="badge text-bg-success border border-success" property="rdfs:label"></span>`
      : this.model.id === 'v-bpa:PartlyJustified'
      ? html`<span class="badge text-bg-warning border border-warning" property="rdfs:label"></span>`
      : this.model.id === 'v-bpa:NotJustified'
      ? html`<span class="badge text-bg-danger border border-danger" property="rdfs:label"></span>`
      : ''
    );
  }
}

customElements.define(ProcessRelevanceIndicator.tag, ProcessRelevanceIndicator);

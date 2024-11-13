import {Component, html} from 'veda-client';

export default class ProcessRelevanceIndicator extends Component(HTMLElement) {
  static tag = 'bpa-process-relevance-indicator';

  property = this.getAttribute('property');

  render() {
    return (
      this.model.id === 'v-bpa:CompletelyJustified'
      ? html`<i class="bi bi-check-circle-fill text-success"></i>&nbsp;<strong class="text-success" property="${this.property}"></strong>`
      : this.model.id === 'v-bpa:PartlyJustified'
      ? html`<i class="bi bi-exclamation-circle-fill text-warning"></i>&nbsp;<strong class="text-warning" property="${this.property}"></strong>`
      : this.model.id === 'v-bpa:NotJustified'
      ? html`<i class="bi bi-dash-circle-fill text-danger"></i>&nbsp;<strong class="text-danger" property="${this.property}"></strong>`
      : html`<i class="bi bi-question-circle-fill text-secondary"></i>&nbsp;<strong class="text-secondary" property="${this.property}"></strong>`
    );
  }
}

customElements.define(ProcessRelevanceIndicator.tag, ProcessRelevanceIndicator);

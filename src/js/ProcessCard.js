import {Component, html} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator';

export default class ProcessCard extends Component(HTMLElement) {
  static tag = 'bpa-process-card';

  render() {
    const laborCosts = this.model['v-bpa:laborCosts']?.[0];
    const processFrequency = this.model['v-bpa:processFrequency']?.[0];

    return html`
      <style>
        a:hover > .business-process-card {
          background-color: #e9e9e9;
        }
        a > .business-process-card {
          background-color: #f5f5f5;
          border: 1px solid #e9e9e9;
        }
      </style>
      <a href="#/ProcessView/${this.model.id}" style="text-decoration: none;">
        <div class="business-process-card card">
          <div class="card-body position-relative">
            <span class="position-absolute top-0 end-0 m-2">
              <span class="badge text-bg-light">${laborCosts && processFrequency ? (laborCosts * processFrequency).toFixed(2) : '0.00'}</span>&nbsp;
              <span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </span>
            <h5 class="mb-0" property="rdfs:label"></h5>
            <span class="text-muted" property="v-bpa:processDescription"></span>
            <div class="mt-2 d-flex justify-content-between align-items-center">
              <div>
                <span rel="v-bpa:hasProcessJustification">
                  <${ProcessJustificationIndicator} class="me-2" about="{{this.model.id}}" property="rdfs:label"></${ProcessJustificationIndicator}>
                </span>
                <span class="badge text-bg-secondary border border-secondary me-2" property="v-bpa:responsibleDepartment"></span>
                <span class="badge text-bg-light border border-secondary me-2 text-muted">
                  <i class="bi bi-arrow-repeat me-1"></i>
                  <span property="v-bpa:processFrequency"></span>&nbsp;
                  <span about="v-bpa:TimesPerYear" property="rdfs:label"></span>
                </span>
              </div>
              <small property="v-bpa:processParticipant"></small>
            </div>
          </div>
        </div>
      </a>
    `;
  }
}

customElements.define(ProcessCard.tag, ProcessCard);

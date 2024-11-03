import {Component, html} from 'veda-client';

export default class BusinessProcessCard extends Component(HTMLElement) {
  static tag = 'bpa-process-card';

  async render() {
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
      <a href="#/BusinessProcessView/${this.model.id}" style="text-decoration: none;">
        <div class="business-process-card card">
          <div class="card-body position-relative">
            <span class="position-absolute top-0 end-0 m-2">
              <span class="badge text-bg-light">${this.model['v-bpa:processFrequency'][0] * this.model['v-bpa:laborCosts'][0]}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span></span>
            </span>
            <h5 class="mb-0" property="rdfs:label"></h5>
            <span class="text-muted" property="v-bpa:processDescription"></span>
            <div class="mt-3 d-flex justify-content-between align-items-center">
              <div>
                <span rel="v-bpa:processRelevance">
                ${this.model['v-bpa:processRelevance'][0].id === 'v-bpa:CompletelyJustified' ? html`
                  <span class="badge text-bg-success border border-success me-2" property="rdfs:label"></span>
                ` : this.model['v-bpa:processRelevance'][0].id === 'v-bpa:PartlyJustified' ? html`
                  <span class="badge text-bg-warning border border-warning me-2" property="rdfs:label"></span>
                ` : html`
                  <span class="badge text-bg-danger border border-danger me-2" property="rdfs:label"></span>
                  `}
                </span>
                <span class="badge text-bg-secondary border border-secondary me-2" property="v-bpa:responsibleDepartment"></span>
                <span class="badge text-bg-light border border-secondary me-2 text-muted">
                  <i class="bi bi-arrow-repeat me-1"></i>
                  <span property="v-bpa:processFrequency"></span>
                  &nbsp;<span about="v-bpa:TimesPerYear" property="rdfs:label"></span>
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

customElements.define(BusinessProcessCard.tag, BusinessProcessCard);

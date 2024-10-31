import {Component, html} from 'veda-client';

export default class BusinessProcessCard extends Component(HTMLElement) {
  static tag = 'bpa-process-card';

  async render() {
    return html`
      <div class="card border-light mb-3" style="background-color: #f5f5f5;">
        <div class="card-body position-relative">
          <span class="position-absolute top-0 end-0 m-2">
            <span class="badge text-bg-warning">${this.model['v-bpa:processFrequency'][0] * this.model['v-bpa:laborCosts'][0]} ч/год</span>
          </span>
          <h5 class="mb-0" property="rdfs:label"></h5>
          <span class="text-muted" property="v-bpa:processDescription"></span>
          <div class="mt-3 d-flex justify-content-between align-items-center">
            <div>
              <span class="badge text-bg-secondary border border-secondary me-2" property="v-bpa:responsibleDepartment"></span>
              <span class="badge text-bg-light border border-secondary me-2">
                <i class="bi bi-arrow-repeat me-1"></i>
                <span property="v-bpa:processFrequency"></span>
                раз в год
              </span>
            </div>
            <small property="v-bpa:processParticipant"></small>
          </div>
        </div>
      </div>
    `;
  }
}

customElements.define(BusinessProcessCard.tag, BusinessProcessCard);

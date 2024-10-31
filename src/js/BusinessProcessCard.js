import {Component, html} from 'veda-client';

export default class BusinessProcessCard extends Component(HTMLElement) {
  static tag = 'bpa-process-card';

  async render() {
    return html`
      <div class="card border-light mb-3" style="background-color: #f5f5f5;">
        <div class="card-body">
          <h5 property="rdfs:label"></h5>
          <span property="v-bpa:processDescription"></span>
        </div>
      </div>
    `;
  }
}

customElements.define(BusinessProcessCard.tag, BusinessProcessCard);

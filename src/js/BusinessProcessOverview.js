import {Component, html} from 'veda-client';

export default class BusinessProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  async render() {
    return html`
      <div class="card border-light mb-3" style="background-color: #f2f2f2;">
        <div class="card-body">
          <h5 property="rdfs:label"></h5>
          <span property="v-bpa:processDescription"></span>
        </div>
      </div>
    `;
  }
}

customElements.define(BusinessProcessOverview.tag, BusinessProcessOverview);

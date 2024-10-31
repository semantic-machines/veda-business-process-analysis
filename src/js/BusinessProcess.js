import {Component, html} from 'veda-client';

export default class BusinessProcess extends Component(HTMLElement) {
  static tag = 'bpa-process';

  
  async render() {
    console.log(this.model);
    return html`
      <style>
        .card {
          background-color: #f2f2f2 !important;
        }
      </style>
      <div class="card text-bg-light border-light mb-3">
        <div class="card-body">
          <h5 property="rdfs:label"></h5>
          <span property="v-bpa:processDescription"></span>
        </div>
      </div>
    `;
  }
}

customElements.define(BusinessProcess.tag, BusinessProcess);

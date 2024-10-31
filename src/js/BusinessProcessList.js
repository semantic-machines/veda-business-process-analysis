import {Component, html, Backend, Model} from 'veda-client';
import BusinessProcessCard from './BusinessProcessCard.js';

export default class BusinessProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
    const {rows: processes} = await Backend.stored_query(params);
    this.processes = processes;
  }
    
  async render() {
    return html`
      <div class="sheet">
        <div class="d-flex justify-content-between align-items-center">
          <div>
            <h3 about="v-bpa:BusinessProcesses" property="rdfs:label"></h3>
          </div>
          <div class="text-end"> 
            <strong about="v-bpa:TotalTimeEffort" property="rdfs:label"></strong>
            <p class="text-muted mb-0">
              ${this.processes.reduce((acc, [,processTime]) => acc + processTime, 0)}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
        </div>
        <hr>
        ${this.processes.map(([processId]) => html`
          <${BusinessProcessCard} about=${processId}></${BusinessProcessCard}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(BusinessProcessList.tag, BusinessProcessList);

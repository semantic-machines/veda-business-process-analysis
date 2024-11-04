import {Component, html, Backend, Model} from 'veda-client';
import ProcessCard from './ProcessCard.js';

let queryResult;

export default class ProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    if (!queryResult) {
      const params = new Model;
      params['rdf:type'] = 'v-s:QueryParams';
      params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
      const {rows: processes} = await Backend.stored_query(params);
      queryResult = processes;
    }
    this.processes = queryResult;
  }

  async render() {
    return html`
      <div class="sheet">
        <div class="d-flex justify-content-between align-items-center mb-4">
          <div class="d-flex justify-content-start align-items-center">
            <div class="me-3 fs-1">
              <i class="bi bi-diagram-3"></i>
            </div>
            <div>
              <h3 class="mb-1">
                <span about="v-bpa:BusinessProcesses" property="rdfs:label"></span>
              </h3>
              <h5 class="mb-0">
                <span about="v-bpa:PoorlyJustified" property="rdfs:label"></span>&nbsp;
                <span class="badge bg-danger">${this.processes.reduce((acc, [,,relevance]) => acc + (relevance === 'v-bpa:NotJustified' ? 1 : 0), 0)}</span>
              </h5>
            </div>
          </div>
          <div class="text-end ps-2"> 
            <strong about="v-bpa:TotalTimeEffort" property="rdfs:label"></strong>
            <p class="text-muted mb-0 mt-1">
              ${this.processes.reduce((acc, [,processTime]) => acc + processTime, 0)}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
        </div>
        <div class="d-flex flex-column gap-3">
          ${this.processes.map(([processId]) => html`
            <${ProcessCard} about=${processId}></${ProcessCard}>
          `).join('')}
        </div>
      </div>
    `;
  }
}

customElements.define(ProcessList.tag, ProcessList);

import {Component, html, Backend, Model} from 'veda-client';
import ProcessCard from './ProcessCard.js';
import ProcessRelevanceIndicator from './ProcessRelevanceIndicator.js';

export default class ProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
    const {rows: processes} = await Backend.stored_query(params);
    this.processes = processes;
  }

  goToProcess(event) {
    const id = event.target.closest('tr').dataset.about;
    location.hash = `#/ProcessView/${id}`;
  }

  render() {
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
                <span class="align-bottom me-2" about="v-bpa:CompletelyJustified" property="rdfs:comment"></span>
                <span class="badge bg-success align-top me-4">${this.processes.reduce((acc, [,,,relevance]) => acc + (relevance === 'v-bpa:CompletelyJustified' ? 1 : 0), 0)}</span>
                <span class="align-bottom me-2" about="v-bpa:PartlyJustified" property="rdfs:comment"></span>
                <span class="badge bg-warning align-top me-4">${this.processes.reduce((acc, [,,,relevance]) => acc + (relevance === 'v-bpa:PartlyJustified' ? 1 : 0), 0)}</span>
                <span class="align-bottom me-2" about="v-bpa:NotJustified" property="rdfs:comment"></span>
                <span class="badge bg-danger align-top me-4">${this.processes.reduce((acc, [,,,relevance]) => acc + (relevance === 'v-bpa:NotJustified' ? 1 : 0), 0)}</span>
              </h5>
            </div>
          </div>
          <div class="text-end ps-2"> 
            <strong about="v-bpa:TotalTimeEffort" property="rdfs:label"></strong>
            <p class="text-muted mb-0 mt-1">
              ${this.processes.reduce((acc, [,,,,,,processTime]) => acc + processTime, 0)}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
        </div>
        <hr>
        <div class="table-responsive">
          <table class="table table-hover mb-4">
            <thead>
              <tr>
                <th width="40%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:processRelevance" property="rdfs:label"></th>
                <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                <th width="15%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                <th width="15%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span>,&nbsp;<span class="text-muted" about="v-bpa:HoursPerYear" property="rdfs:label"></span></th>
              </tr>
            </thead>
            <tbody>
              ${this.processes.map(([id, label, description, relevance, responsibleDepartment, processParticipant, laborCosts]) => html`
                <tr @click="goToProcess" data-about="${id}">
                  <td><h5 class="mb-0">${label}</h5><p class="text-muted mb-0">${description && description.length > 50 ? description.slice(0, 50) + '...' : description}</p></td>
                  <td><${ProcessRelevanceIndicator} about="${relevance}" property="rdfs:label"></${ProcessRelevanceIndicator}></td>
                  <td>${responsibleDepartment}</td>
                  <td>${processParticipant}</td>
                  <td><strong>${laborCosts ?? 0}</strong></td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }
}

customElements.define(ProcessList.tag, ProcessList);

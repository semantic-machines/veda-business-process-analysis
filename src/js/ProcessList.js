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
        <div class="d-flex justify-content-between align-items-center">
          <div class="d-flex justify-content-start align-items-center">
            <div class="me-3 fs-1">
              <i class="bi bi-diagram-3"></i>
            </div>
            <div>
              <h3 class="mb-1">
                <span about="v-bpa:BusinessProcesses" property="rdfs:label"></span>
              </h3>
            </div>
          </div>
          <button type="button" class="btn btn-light" data-bs-toggle="modal" data-bs-target="#filters">
            <i class="bi bi-chevron-down"></i>
            Фильтры
          </button>
        </div>
        <div class="table-responsive">
          <table class="table table-hover mb-4">
            <thead>
              <tr>
                <th width="50%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:processRelevance" property="rdfs:label"></th>
                <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
              </tr>
            </thead>
            <tbody>
              ${this.processes.map(([id, label, description, relevance, responsibleDepartment, processParticipant, laborCosts]) => html`
                <tr @click="goToProcess" data-about="${id}">
                  <td class="align-middle"><h5 class="mb-0">${label}</h5><p class="text-muted mb-0">${description && description.length > 60 ? description.slice(0, 60) + '...' : description}</p></td>
                  <td class="align-middle"><${ProcessRelevanceIndicator} about="${relevance}" property="rdfs:label"></${ProcessRelevanceIndicator}></td>
                  <td class="align-middle">${responsibleDepartment}</td>
                  <td class="align-middle"><i class="bi bi-people-fill me-1"></i>${processParticipant && typeof processParticipant === 'string' ? processParticipant.split(',').length : 0}</td>
                  <td class="align-middle"><strong>${laborCosts ?? 0}</strong><br><span class="text-muted" about="v-bpa:HoursPerYear" property="rdfs:comment"></span></td>
                </tr>
              `).join('')}
            </tbody>
          </table>

          <div class="modal fade" id="filters" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
            <div class="modal-dialog modal-dialog-centered">
              <div class="modal-content">
                <div class="modal-header">
                  <h1 class="modal-title fs-5" id="staticBackdropLabel" about="v-bpa:Filters" property="rdfs:label"></h1>
                  <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                </div>
                <div class="modal-body">
                  <button type="button" class="btn btn-secondary me-2" about="v-bpa:ApplyFilters" property="rdfs:label"></button>
                  <button type="button" class="btn btn-light" about="v-bpa:ResetFilters" property="rdfs:label"></button>
                </div>
              </div>
            </div>
          </div>        
        </div>
      </div>
    `;
  }
}

customElements.define(ProcessList.tag, ProcessList);

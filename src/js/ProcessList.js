import {Component, html, Backend, Model} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Literal from './Literal.js';

export default class ProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
    const {rows: processes} = await Backend.stored_query(params);
    this.processes = processes;
    this.filtersData = null;
    this.filtered = this.processes;
  }

  goToProcess(event) {
    const id = event.target.closest('tr').dataset.about;
    location.hash = `#/ProcessView/${id}`;
  }

  applyFilters(event) {
    event.preventDefault();
    const form = event.target.closest('form');
    const formData = new FormData(form);
    const data = Object.fromEntries(formData.entries());
    this.filtersData = data;

    this.filtered = this.processes.filter(([id, label, description, relevance, responsibleDepartment, processParticipant, laborCosts]) => {
      // Фильтр по названию
      if (data['rdfs:label'] && !label.toLowerCase().includes(data['rdfs:label'].toLowerCase())) {
        return false;
      }
      // Фильтр по релевантности
      if (data['v-bpa:hasProcessJustification'] && relevance !== data['v-bpa:hasProcessJustification']) {
        return false;
      }
      // Фильтр по ответственному подразделению
      if (data['v-bpa:responsibleDepartment'] && 
          !responsibleDepartment.toLowerCase().includes(data['v-bpa:responsibleDepartment'].toLowerCase())) {
        return false;
      }
      // Фильтр по количеству участников
      const participantsCount = processParticipant && typeof processParticipant === 'string' 
        ? processParticipant.split(',').length 
        : 0;
      if (data.participantsCountFrom && participantsCount < Number(data.participantsCountFrom)) {
        return false;
      }
      if (data.participantsCountTo && participantsCount > Number(data.participantsCountTo)) {
        return false;
      }
      // Фильтр по трудозатратам
      const costs = laborCosts ?? 0;
      if (data.laborCostsFrom && costs < Number(data.laborCostsFrom)) {
        return false;
      }
      if (data.laborCostsTo && costs > Number(data.laborCostsTo)) {
        return false;
      }
      return true;
    });

    this.renderFilteredProcesses();
    this.renderFiltersCount();
  }

  resetFilters() {
    this.filtersData = null;
    this.filtered = this.processes;
  }

  renderFilteredProcesses() {
    const container = this.querySelector('#filtered-processes');
    container.innerHTML = `
      ${this.filtered.map(([id, label, description, justification, responsibleDepartment, processParticipant, laborCosts]) => html`
        <tr onclick="location.hash = '#/ProcessView/${id}'">
          <td class="align-middle"><h5 class="mb-0">${label}</h5><p class="text-muted mb-0">${description && description.length > 60 ? description.slice(0, 60) + '...' : description}</p></td>
          <td class="align-middle"><${ProcessJustificationIndicator} about="${justification}" property="rdfs:label"></${ProcessJustificationIndicator}></td>
          <td class="align-middle">${responsibleDepartment}</td>
          <td class="align-middle"><i class="bi bi-people-fill me-1"></i>${processParticipant && typeof processParticipant === 'string' ? processParticipant.split(',').length : 0}</td>
          <td class="align-middle lh-sm">
            <strong>${laborCosts ?? 0}</strong><br>
            <small><${Literal} class="text-secondary" about="v-bpa:HoursPerYear" property="rdfs:comment"></${Literal}></small>
          </td>
        </tr>
      `).join('')}
    `;
  }

  renderFiltersCount() {
    const button = this.querySelector('#filters-button');
    const count = this.filtersData ? Object.values(this.filtersData).filter(value => value).length || null : null;
    button.lastElementChild.textContent = count ?? '';
  }

  post() {
    this.renderFilteredProcesses();
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
          <button type="button" class="btn btn-light" data-bs-toggle="modal" data-bs-target="#filters" id="filters-button">
            <i class="bi bi-chevron-down me-1"></i>
            <span about="v-bpa:Filters" property="rdfs:label"></span>
            <span class="badge rounded-pill bg-danger ms-1"></span>
          </button>
        </div>
        <div class="table-responsive">
          <style>
            #processes-table tbody tr:last-child {
              border-bottom: 1px solid transparent;
            }
          </style>
          <table class="table table-hover mb-0" id="processes-table">
            <thead>
              <tr>
                <th width="50%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:hasProcessJustification" property="rdfs:label"></th>
                <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
              </tr>
            </thead>
            <tbody id="filtered-processes"></tbody>
          </table>
          <div class="modal" id="filters" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
            <div class="modal-dialog modal-dialog-centered">
              <div class="modal-content">
                <div class="modal-header">
                  <h1 class="modal-title fs-5" id="staticBackdropLabel" about="v-bpa:Filters" property="rdfs:label"></h1>
                  <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                </div>
                <div class="modal-body">
                  <form @submit="applyFilters">
                    <div class="mb-5">
                      <div class="mb-3">
                        <label for="label" class="form-label" about="rdfs:label" property="rdfs:label"></label>
                        <input type="text" class="form-control" id="label" name="rdfs:label">
                      </div>
                      <div class="mb-3">
                        <label for="justification" class="form-label" about="v-bpa:hasProcessJustification" property="rdfs:label"></label>
                        <select class="form-select" id="justification" name="v-bpa:hasProcessJustification">
                          <option value="">---</option>
                          <option value="v-bpa:CompletelyJustified" about="v-bpa:CompletelyJustified" property="rdfs:label"></option>
                          <option value="v-bpa:PartlyJustified" about="v-bpa:PartlyJustified" property="rdfs:label"></option>
                          <option value="v-bpa:PoorlyJustified" about="v-bpa:PoorlyJustified" property="rdfs:label"></option>
                          <option value="v-bpa:NoDocumentForJustification" about="v-bpa:NoDocumentForJustification" property="rdfs:label"></option>
                        </select>
                      </div>
                      <div class="mb-3">
                        <label for="responsibleDepartment" class="form-label" about="v-bpa:responsibleDepartment" property="rdfs:comment"></label>
                        <input type="text" class="form-control" id="responsibleDepartment" name="v-bpa:responsibleDepartment">
                      </div>
                      <div class="mb-3">
                        <label class="form-label me-2" about="v-bpa:processParticipant" property="rdfs:comment"></label>
                        <div class="mb-3 d-flex align-items-center" id="participantsCount">
                          <input type="number" placeholder="от" class="form-control me-2 w-25" name="participantsCountFrom">
                          <input type="number" placeholder="до" class="form-control w-25" name="participantsCountTo">
                        </div>
                      </div>
                      <div class="mb-3">
                        <label class="form-label" for="laborCosts" about="v-bpa:laborCosts" property="rdfs:label"></label>
                        <div class="mb-3 d-flex align-items-center" id="laborCosts">
                          <input type="number" placeholder="от" class="form-control me-2 w-25" name="laborCostsFrom">
                          <input type="number" placeholder="до" class="form-control w-25" name="laborCostsTo">
                        </div>
                      </div>
                    </div>
                    <button type="submit" class="btn btn-secondary me-2" data-bs-dismiss="modal"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
                    <button type="reset" @click="resetFilters" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
                  </form>
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

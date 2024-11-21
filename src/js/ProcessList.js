import {Component, html, Backend, Model} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Literal from './Literal.js';
import InputAudio from './controls/InputAudio.js';
import {Modal} from 'bootstrap';

class ProcessFilters extends Component(HTMLElement) {
  static tag = 'bpa-process-filters';

  data = {};

  applyFilters(e) {
    e.preventDefault();
    const form = e.target.closest('form');
    const formData = new FormData(form);
    for (const key of formData.keys()) {
      this.data[key] = formData.getAll(key);
    }
    console.log(JSON.stringify(this.data, null, 2));

    this.renderFiltersCount();
    this.dispatchEvent(new CustomEvent('filters-changed', {detail: this.data}));
  }

  resetFilters() {
    this.data = {};
    this.renderFiltersCount();
    this.dispatchEvent(new CustomEvent('filters-changed', {detail: null}));
  }

  render() {
    return html`
      <button type="button" class="btn btn-link text-dark text-decoration-none" data-bs-toggle="modal" data-bs-target="#filters" id="filters-button">
        <i class="bi bi-chevron-down me-1"></i>
        <span about="v-bpa:Filters" property="rdfs:label"></span>
        <span class="badge rounded-pill bg-danger ms-1"></span>
      </button>
      <div class="modal fade" id="filters" data-bs-keyboard="true" tabindex="-1" aria-hidden="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-header">
              <h1 class="modal-title fs-5" id="staticBackdropLabel" about="v-bpa:Filters" property="rdfs:label"></h1>
              <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body">
              <form @submit="${(e) => this.applyFilters(e)}">
                <div class="mb-5">
                  <div class="mb-3">
                    <label for="label" class="form-label" about="rdfs:label" property="rdfs:label"></label>
                    <input type="text" class="form-control" id="label" name="rdfs:label">
                  </div>
                  <div class="mb-3">
                    <label for="justification" class="form-label" about="v-bpa:hasProcessJustification" property="rdfs:label"></label>
                    <select class="form-select" id="justification" name="v-bpa:hasProcessJustification" multiple>
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
                    <label class="form-label" for="laborCosts" about="v-bpa:laborCosts" property="rdfs:label"></label>
                    <div class="mb-3 d-flex align-items-center" id="laborCosts">
                      <input type="number" placeholder="от" class="form-control me-2 w-25" name="v-bpa:laborCosts">
                      <input type="number" placeholder="до" class="form-control w-25" name="v-bpa:laborCosts">
                    </div>
                  </div>
                  <div class="mb-3 position-relative">
                    <label for="process-filter-raw-input" class="form-label" about="v-bpa:rawInput" property="rdfs:label"></label>
                    <textarea class="form-control" id="process-filter-raw-input" name="v-bpa:rawInput" rows="3"></textarea>
                    <div class="position-absolute bottom-0" style="right:0.75rem;">
                      <${InputAudio} data-for="process-filter-raw-input"></${InputAudio}>
                    </div>
                  </div>
                </div>
                <button type="submit" class="btn btn-secondary me-2" data-bs-dismiss="modal"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
                <button type="reset" @click="${(e) => this.resetFilters(e)}" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
              </form>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  renderFiltersCount() {
    const button = this.querySelector('#filters-button');
    const count = this.data ? Object.values(this.data).filter(value => value.some(v => v)).length || null : null;
    button.lastElementChild.textContent = count ?? '';
  }

  post() {
    this.querySelector('#filters').addEventListener('shown.bs.modal', () => {
      this.querySelector('.btn-close')?.focus();
    });
  }

  removed() {
    Modal.getInstance(this.lastElementChild)?.hide();
  }
}

customElements.define(ProcessFilters.tag, ProcessFilters);

export default class ProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
    params['v-s:resultFormat'] = 'rows';
    const {rows: processes} = await Backend.stored_query(params);
    this.processes = processes;
    this.filtersData = null;
    this.filtered = this.processes;
  }

  goToProcess(event) {
    const id = event.target.closest('tr').dataset.about;
    location.hash = `#/ProcessView/${id}`;
  }

  handleFiltersChange = (event) => {
    this.filtersData = event.detail;
    if (!this.filtersData) {
      this.filtered = this.processes;
    } else {
      this.filtered = this.processes.filter(([id, label, description, relevance, responsibleDepartment, processParticipant, laborCosts]) => {
        // Фильтр по названию
        if (this.filtersData['rdfs:label'] && this.filtersData['rdfs:label'][0] && !label.toLowerCase().includes(this.filtersData['rdfs:label'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по релевантности
        if (this.filtersData['v-bpa:hasProcessJustification'] && this.filtersData['v-bpa:hasProcessJustification'].length && !this.filtersData['v-bpa:hasProcessJustification'].includes(relevance)) {
          return false;
        }
        // Фильтр по ответственному подразделению
        if (this.filtersData['v-bpa:responsibleDepartment'] && this.filtersData['v-bpa:responsibleDepartment'][0] &&
            !responsibleDepartment.toLowerCase().includes(this.filtersData['v-bpa:responsibleDepartment'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по трудозатратам
        const costs = laborCosts ?? 0;
        if (this.filtersData['v-bpa:laborCosts'] && this.filtersData['v-bpa:laborCosts'][0] && costs < Number(this.filtersData['v-bpa:laborCosts'][0])) {
          return false;
        }
        if (this.filtersData['v-bpa:laborCosts'] && this.filtersData['v-bpa:laborCosts'][1] && costs > Number(this.filtersData['v-bpa:laborCosts'][1])) {
          return false;
        }
        return true;
      });
    }
    this.renderFilteredProcesses();
  }

  renderFilteredProcesses() {
    const container = this.querySelector('#filtered-processes');
    container.innerHTML = `
      ${this.filtered.map(([id, label, description, justification, responsibleDepartment, processParticipant, laborCosts]) => html`
        <tr onclick="location.hash = '#/ProcessView/${id}'">
          <td class="align-middle"><h5 class="mb-0">${label}</h5><p class="text-muted mb-0">${description && description.length > 60 ? description.slice(0, 60) + '...' : description}</p></td>
          <td class="align-middle"><${ProcessJustificationIndicator} class="text-nowrap" about="${justification}" property="rdfs:label"></${ProcessJustificationIndicator}></td>
          <td class="align-middle">${responsibleDepartment}</td>
          <td class="align-middle">
            <i class="bi bi-people-fill me-1"></i>
            <strong>${processParticipant && typeof processParticipant === 'string' ? processParticipant.split(',').length : 0}</strong>
          </td>
          <td class="align-middle lh-sm">
            <strong>${laborCosts ?? 0}</strong><br>
            <small><${Literal} class="text-secondary" about="v-bpa:HoursPerYear" property="rdfs:comment"></${Literal}></small>
          </td>
        </tr>
      `).join('')}
    `;
  }

  post() {
    this.renderFilteredProcesses();
    this.querySelector(`${ProcessFilters}`).addEventListener('filters-changed', this.handleFiltersChange);
  }

  render() {
    return html`
      <div class="sheet">
        <div class="d-flex align-items-center">
          <i class="bi bi-diagram-3 ms-2 me-3 fs-1"></i>
          <h3 class="mb-1" about="v-bpa:BusinessProcesses" property="rdfs:label"></h3>
          <${ProcessFilters} class="ms-auto"></${ProcessFilters}>
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
        </div>
      </div>
    `;
  }
}

customElements.define(ProcessList.tag, ProcessList);

import {Component, html, safe, Backend, Model} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Literal from './Literal.js';
import ProcessQuickCreateModal from './ProcessQuickCreateModal.js';
import ProcessFiltersModal from './ProcessFiltersModal.js';

export default class ProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    this.poorlyJustified = Number(this.getAttribute('poorly-justified') || 0);
    this.noDocument = Number(this.getAttribute('no-document') || 0);

    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcessesOrderedByDepartment';
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
        if (this.filtersData['rdfs:label_filter'] && this.filtersData['rdfs:label_filter'][0] && !label.toLowerCase().includes(this.filtersData['rdfs:label_filter'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по релевантности
        if (this.filtersData['v-bpa:hasProcessJustification_filter'] && this.filtersData['v-bpa:hasProcessJustification_filter'].length && !this.filtersData['v-bpa:hasProcessJustification_filter'].includes(relevance)) {
          return false;
        }
        // Фильтр по ответственному подразделению
        if (this.filtersData['v-bpa:responsibleDepartment_filter'] && this.filtersData['v-bpa:responsibleDepartment_filter'][0] &&
            !responsibleDepartment.toLowerCase().includes(this.filtersData['v-bpa:responsibleDepartment_filter'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по трудозатратам
        const costs = laborCosts ?? 0;
        if (this.filtersData['v-bpa:laborCosts_filter'] && this.filtersData['v-bpa:laborCosts_filter'][0] && costs < Number(this.filtersData['v-bpa:laborCosts_filter'][0])) {
          return false;
        }
        if (this.filtersData['v-bpa:laborCosts_filter'] && this.filtersData['v-bpa:laborCosts_filter'][1] && costs > Number(this.filtersData['v-bpa:laborCosts_filter'][1])) {
          return false;
        }
        return true;
      });
    }
    this.renderFilteredProcesses();
  }

  renderFilteredProcesses() {
    const container = this.querySelector('#filtered-processes');
    const fragment = document.createDocumentFragment();

    let currentDepartment = '';

    this.filtered.forEach(([...values]) => {
      const [id, label, description, justification, responsibleDepartment, processParticipant, laborCosts] = values.map(safe);

      if (responsibleDepartment !== currentDepartment) {
        currentDepartment = responsibleDepartment;
        const departmentRow = document.createElement('tr');
        departmentRow.className = 'table-secondary';
        departmentRow.innerHTML = `
          <td colspan="5" class="fw-bold text-secondary rounded-bottom">${responsibleDepartment || 'Без отдела'}</td>
        `;
        fragment.appendChild(departmentRow);
      }

      const row = document.createElement('tr');
      row.onclick = () => location.hash = `#/ProcessView/${id}`;
      row.innerHTML = `
        <td class="align-middle"><h5 class="mb-0">${label}</h5><p class="text-muted mb-0">${description && description.length > 60 ? description.slice(0, 60) + '...' : description}</p></td>
        <td class="align-middle"><${ProcessJustificationIndicator} class="text-nowrap" about="${justification}" property="rdfs:label"></${ProcessJustificationIndicator}></td>
        <td class="align-middle">${responsibleDepartment}</td>
        <td class="align-middle text-end">
          <i class="bi bi-people-fill me-1"></i>
          <strong>${processParticipant && typeof processParticipant === 'string' ? processParticipant.split(',').length : 0}</strong>
        </td>
        <td class="align-middle text-end lh-sm">
          <strong>${laborCosts ?? 0}</strong><br>
          <small><${Literal} class="text-secondary" about="v-bpa:HoursPerYear" property="rdfs:comment"></${Literal}></small>
        </td>
      `;
      fragment.appendChild(row);
    });

    container.innerHTML = '';
    container.appendChild(fragment);
  }

  post() {
    this.renderFilteredProcesses();
    this.querySelector(`${ProcessFiltersModal}`).addEventListener('filters-changed', this.handleFiltersChange);
  }

  render() {
    return html`
      <div class="sheet">
        <div class="d-flex align-items-center">
          <i class="bi bi-diagram-3 ms-2 me-3 fs-1"></i>
          <h3 class="mb-1 me-5" about="v-bpa:BusinessProcesses" property="rdfs:label"></h3>
          ${this.poorlyJustified
            ? html`<strong class="text-danger me-2" about="v-bpa:PoorlyJustified" property="rdfs:comment"></strong><span class="badge bg-danger">${this.poorlyJustified}</span>`
            : this.noDocument
            ? html`<strong class="text-secondary me-2" about="v-bpa:NoDocumentForJustification" property="rdfs:comment"></strong><span class="badge bg-secondary">${this.noDocument}</span>`
            : ''
          }
          <div class="d-flex align-items-center ms-auto">
            <a href="#process-quick-create-modal" data-bs-toggle="modal" data-bs-target="#process-quick-create-modal" class="btn btn-link text-dark text-decoration-none me-3">
              <i class="bi bi-plus me-1"></i>
              <span about="v-bpa:AddProcess" property="rdfs:label"></span>
            </a>
            <${ProcessQuickCreateModal}></${ProcessQuickCreateModal}>
            <${ProcessFiltersModal}></${ProcessFiltersModal}>
          </div>
        </div>
        <div class="table-responsive">
          <style>
            #processes-table tbody tr:last-child {
              border-bottom: 1px solid transparent;
            }
            #processes-table td {
              cursor: pointer;
            }
            #processes-table tr.table-secondary > td {
              cursor: default;
              border-bottom: 1px solid transparent;
            }
            #processes-table tr.table-secondary:hover > td {
              box-shadow: none;
            }
          </style>
          <table class="table table-hover mb-0" id="processes-table">
            <thead>
              <tr>
                <th width="50%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:hasProcessJustification" property="rdfs:label"></th>
                <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal text-end" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal text-end"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
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

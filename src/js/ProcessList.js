import {Component, html, safe, Backend, Model, timeout} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Literal from './Literal.js';
import InputAudio from './controls/InputAudio.js';
import {Modal} from 'bootstrap';

class ProcessFilterForm extends Component(HTMLElement) {
  static tag = 'bpa-process-filter-form';

  data = {};

  added() {
    const savedData = sessionStorage.getItem('ProcessFilterForm_data');
    if (savedData) {
      this.data = JSON.parse(savedData);
      this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
    }
  }

  updateDataFromForm() {
    this.data = {};
    const formData = new FormData(this.firstElementChild);
    const formKeys = {
      'rdfs:label': String,
      'v-bpa:hasProcessJustification': Array,
      'v-bpa:responsibleDepartment': String,
      'v-bpa:laborCosts': Array,
      'v-bpa:rawInput': String
    }

    for (const [key, type] of Object.entries(formKeys)) {
      const values = formData.getAll(key);
      this.data[key] = type === Array ? values : values.filter(Boolean);
    }
  }

  submit(e) {
    e.preventDefault();
    this.updateDataFromForm();
    sessionStorage.setItem('ProcessFilterForm_data', JSON.stringify(this.data));
    this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
  }

  reset() {
    sessionStorage.removeItem('ProcessFilterForm_data');
    this.data = {};
    this.update();
    this.dispatchEvent(new CustomEvent('reset'));
  }

  async handleRawInput() {
    this.updateDataFromForm();

    if (!this.data['v-bpa:rawInput'].length) return;

    try {
      this.showSpinner(true);
      await this.createRequest();
    } catch (error) {
      this.handleError(error);
    } finally {
      this.showSpinner(false);
    }
  }

  createRequest = async () => {
    const request = new Model;
    request['rdf:type'] = 'v-bpa:GenericProcessingRequest';
    request['v-bpa:prompt'] = 'v-bpa:ModifySearchFiltersPrompt';
    request['v-bpa:rawInput'] = [...this.data['v-bpa:rawInput']];
    delete this.data['v-bpa:rawInput'];
    request['v-bpa:structuredInput'] = JSON.stringify(this.data);
    request.subscribe();
    await request.save();
    await this.waitForRequestResult(request);
  }

  waitForRequestResult = (request) => {
    return Promise.race([
      this.handleRequestResult(request),
      this.createTimeout()
    ]);
  }

  handleRequestResult = (request) => {
    return new Promise((resolve, reject) => {
      const handleReset = async () => {
        if (!request.hasValue('v-bpa:structuredOutput')) return;

        try {
          this.data = JSON.parse(request['v-bpa:structuredOutput'][0]);
          await this.update();
          resolve();
        } catch (error) {
          reject(error);
        } finally {
          request.off('afterreset', handleReset);
          request.unsubscribe();
        }
      }

      request.on('afterreset', handleReset);
    });
  }

  handleError = (error) => {
    alert(`Ошибка голосового заполнения формы: ${error.message}`);
    console.error('Ошибка голосового заполнения формы', error);
  }

  createTimeout = () => {
    return timeout(30000).then(() => {
      throw new Error('Превышено время ожидания обработки голосового ввода');
    });
  }

  showSpinner(show) {
    const inputAudio = this.querySelector(`${InputAudio}`);
    inputAudio.classList.toggle('d-none', show);
    const spinner = this.querySelector('#process-filter-raw-input-spinner');
    spinner.classList.toggle('d-none', !show);
  }

  render() {
    return html`
      <form @submit="${(e) => this.submit(e)}" @reset="${() => this.reset()}">
        <div class="mb-3">
          <label for="label" class="form-label" about="rdfs:label" property="rdfs:label"></label>
          <input type="text" class="form-control" id="label" name="rdfs:label" value="${this.data['rdfs:label']?.[0] || ''}">
        </div>
        <div class="mb-3">
          <label for="justification" class="form-label" about="v-bpa:hasProcessJustification" property="rdfs:label"></label>
          <select class="form-select" id="justification" name="v-bpa:hasProcessJustification" multiple>
            <option value="v-bpa:CompletelyJustified" about="v-bpa:CompletelyJustified" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification']?.includes('v-bpa:CompletelyJustified') ? 'selected' : ''}></option>
            <option value="v-bpa:PartlyJustified" about="v-bpa:PartlyJustified" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification']?.includes('v-bpa:PartlyJustified') ? 'selected' : ''}></option>
            <option value="v-bpa:PoorlyJustified" about="v-bpa:PoorlyJustified" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification']?.includes('v-bpa:PoorlyJustified') ? 'selected' : ''}></option>
            <option value="v-bpa:NoDocumentForJustification" about="v-bpa:NoDocumentForJustification" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification']?.includes('v-bpa:NoDocumentForJustification') ? 'selected' : ''}></option>
          </select>
        </div>
        <div class="mb-3">
          <label for="responsibleDepartment" class="form-label" about="v-bpa:responsibleDepartment" property="rdfs:comment"></label>
          <input type="text" class="form-control" id="responsibleDepartment" name="v-bpa:responsibleDepartment" value="${this.data['v-bpa:responsibleDepartment']?.[0] || ''}">
        </div>
        <div class="mb-3">
          <label class="form-label" for="laborCosts" about="v-bpa:laborCosts" property="rdfs:label"></label>
          <div class="mb-3 d-flex align-items-center" id="laborCosts">
            <input type="number" placeholder="от" class="form-control me-2 w-25" name="v-bpa:laborCosts" value="${this.data['v-bpa:laborCosts']?.[0] || ''}">
            <input type="number" placeholder="до" class="form-control w-25" name="v-bpa:laborCosts" value="${this.data['v-bpa:laborCosts']?.[1] || ''}">
          </div>
        </div>
        <div class="d-flex justify-content-between">
          <button type="submit" class="btn btn-secondary me-2"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
          <button type="reset" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
          <div class="mb-3 position-relative ms-auto">
            <input type="hidden" class="form-control" id="process-filter-raw-input" name="v-bpa:rawInput" @change="${(e) => this.handleRawInput(e)}" value="${this.data['v-bpa:rawInput']?.[0] || ''}">
            <div class="position-absolute" style="bottom: -1rem; right: 0;">
              <${InputAudio} data-for="process-filter-raw-input" id="process-filter-raw-input-audio"></${InputAudio}>
              <div class="d-none spinner-grow spinner-grow-sm" id="process-filter-raw-input-spinner"></div>
            </div>
          </div>
        </div>
      </form>
    `;
  }
}

customElements.define(ProcessFilterForm.tag, ProcessFilterForm);

class ProcessFilters extends Component(HTMLElement) {
  static tag = 'bpa-process-filters';

  data = {};

  handleApplyFilters(data) {
    this.data = data;
    this.renderFiltersCount();
    this.dispatchEvent(new CustomEvent('filters-changed', {detail: this.data}));
    Modal.getInstance(this.lastElementChild)?.hide();
  }

  handleResetFilters() {
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
              <${ProcessFilterForm}
                @apply="${(e) => this.handleApplyFilters(e.detail)}"
                @reset="${() => this.handleResetFilters()}"
              ></${ProcessFilterForm}>
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
    const fragment = document.createDocumentFragment();

    this.filtered.forEach(([...values]) => {
      const [id, label, description, justification, responsibleDepartment, processParticipant, laborCosts] = values.map(safe);
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

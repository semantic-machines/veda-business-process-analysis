import {Component, html, safe, Backend, Model, timeout} from 'veda-client';
import {Modal} from 'bootstrap';
import InputAudio from './controls/InputAudio.js';

class DocumentFilterForm extends Component(HTMLElement) {
  static tag = 'bpa-document-filter-form';

  data = {};

  added() {
    const savedData = sessionStorage.getItem('DocumentFilterForm_data');
    if (savedData) {
      this.data = JSON.parse(savedData);
      this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
    }
  }

  updateDataFromForm() {
    this.data = {};
    const formData = new FormData(this.firstElementChild);
    const formKeys = {
      'v-bpa:documentName': String,
      'v-bpa:documentContent': String,
      'v-s:created': Array,
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
    sessionStorage.setItem('DocumentFilterForm_data', JSON.stringify(this.data));
    this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
  }

  reset() {
    sessionStorage.removeItem('DocumentFilterForm_data');
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
    const spinner = this.querySelector('#document-filter-raw-input-spinner');
    spinner.classList.toggle('d-none', !show);
  }

  render() {
    return html`
      <form @submit="${(e) => this.submit(e)}" @reset="${() => this.reset()}">
        <div class="mb-5">
          <div class="mb-3">
            <label for="name" class="form-label" about="v-bpa:documentName" property="rdfs:label"></label>
            <input type="text" class="form-control" id="name" name="v-bpa:documentName" value="${this.data['v-bpa:documentName']?.[0] || ''}">
          </div>
          <div class="mb-3">
            <label for="content" class="form-label" about="v-bpa:documentContent" property="rdfs:label"></label>
            <input type="text" class="form-control" id="content" name="v-bpa:documentContent" value="${this.data['v-bpa:documentContent']?.[0] || ''}">
          </div>
          <div class="mb-3">
            <label class="form-label me-2" about="v-s:created" property="rdfs:label"></label>
            <div class="mb-3 d-flex align-items-center" id="created">
              <input type="date" placeholder="от" class="form-control me-2 w-50" name="v-s:created" value="${this.data['v-s:created']?.[0] || ''}">
              <input type="date" placeholder="до" class="form-control w-50" name="v-s:created" value="${this.data['v-s:created']?.[1] || ''}">
            </div>
          </div>
          <div class="mb-3 position-relative">
            <textarea class="form-control" rows="3" id="document-filter-raw-input" name="v-bpa:rawInput" @change="${(e) => this.handleRawInput(e)}" value="${this.data['v-bpa:rawInput']?.[0] || ''}"></textarea>
            <div class="position-absolute" style="bottom: 0.125rem; right: 0.5rem;">
              <${InputAudio} data-for="document-filter-raw-input" id="document-filter-raw-input-audio"></${InputAudio}>
              <div class="d-none spinner-grow spinner-grow-sm" id="document-filter-raw-input-spinner"></div>
            </div>
          </div>
        </div>
        <button type="submit" class="btn btn-secondary me-2"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
        <button type="reset" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
      </form>
    `;
  }
}

customElements.define(DocumentFilterForm.tag, DocumentFilterForm);

class DocumentFilters extends Component(HTMLElement) {
  static tag = 'bpa-document-filters';

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

  renderFiltersCount() {
    const button = this.querySelector('#filters-button');
    const count = this.data ? Object.values(this.data).filter(value => value.some(v => v)).length || null : null;
    button.lastElementChild.textContent = count ?? '';
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
              <${DocumentFilterForm}
                @apply="${(e) => this.handleApplyFilters(e.detail)}"
                @reset="${() => this.handleResetFilters()}"
              ></${DocumentFilterForm}>
            </div>
          </div>
        </div>
      </div>
    `;
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

customElements.define(DocumentFilters.tag, DocumentFilters);

export default class DocumentList extends Component(HTMLElement) {
  static tag = 'bpa-document-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessDocuments';
    params['v-s:resultFormat'] = 'rows';
    const {rows: documents} = await Backend.stored_query(params);
    this.documents = documents;
    this.filtersData = null;
    this.filtered = this.documents;
  }

  goToDocument(event) {
    const id = event.target.closest('tr').dataset.about;
    location.hash = `#/DocumentView/${id}`;
  }

  handleFiltersChange = (event) => {
    this.filtersData = event.detail;
    if (!this.filtersData) {
      this.filtered = this.documents;
    } else {
      this.filtered = this.documents.filter(([id, name, content, created]) => {
        // Фильтр по названию
        if (this.filtersData['v-bpa:documentName'] && this.filtersData['v-bpa:documentName'][0] &&
            !name.toLowerCase().includes(this.filtersData['v-bpa:documentName'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по содержанию
        if (this.filtersData['v-bpa:documentContent'] && this.filtersData['v-bpa:documentContent'][0] &&
            !content.toLowerCase().includes(this.filtersData['v-bpa:documentContent'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по дате создания
        created = new Date(created);
        if (this.filtersData['v-s:created'] && this.filtersData['v-s:created'][0]) {
          const from = new Date(this.filtersData['v-s:created'][0]);
          from.setHours(0, 0, 0, 0);
          if (created < from) return false;
        }
        if (this.filtersData['v-s:created'] && this.filtersData['v-s:created'][1]) {
          const to = new Date(this.filtersData['v-s:created'][1]);
          to.setHours(23, 59, 59, 999);
          if (created > to) return false;
        }
        return true;
      });
    }
    this.renderFilteredDocuments();
  }

  renderFilteredDocuments() {
    const container = this.querySelector('#filtered-documents');
    const fragment = document.createDocumentFragment();
    this.filtered.forEach(([id, name, content, created]) => {
      [id, name, content, created] = [id, name, content, created].map(safe);
      const row = document.createElement('tr');
      row.onclick = () => location.hash = `#/DocumentView/${id}`;
      row.innerHTML = `
        <td class="align-middle"><h5 class="mb-0">${name}</h5><p class="text-muted mb-0">${content && content.length > 120 ? content.slice(0, 120) + '...' : content}</p></td>
        <td class="align-middle text-end">${new Date(created).toLocaleDateString()}</td>
      `;
      fragment.appendChild(row);
    });
    container.innerHTML = '';
    container.appendChild(fragment);
  }

  post() {
    this.renderFilteredDocuments();
    this.querySelector('bpa-document-filters').addEventListener('filters-changed', this.handleFiltersChange);
  }

  render() {
    return html`
      <div class="sheet">
        <div class="d-flex align-items-center">
          <i class="bi bi-file-earmark-text ms-2 me-3 fs-1"></i>
          <h3 class="mb-1" about="v-bpa:ProcessDocuments" property="rdfs:label"></h3>
          <${DocumentFilters} class="ms-auto"></${DocumentFilters}>
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
                <th width="100%" class="text-secondary fw-normal" about="v-bpa:documentName" property="rdfs:label"></th>
                <th class="text-secondary fw-normal text-end text-nowrap" about="v-s:created" property="rdfs:label"></th>
              </tr>
            </thead>
            <tbody id="filtered-documents"></tbody>
          </table>
        </div>
      </div>
    `;
  }
}

customElements.define(DocumentList.tag, DocumentList);

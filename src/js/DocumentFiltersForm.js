import {Component, html, Model, timeout} from 'veda-client';
import InputAudio from './controls/InputAudio.js';

export default class DocumentFiltersForm extends Component(HTMLElement) {
  static tag = 'bpa-document-filters-form';

  data = {};

  added() {
    const savedData = sessionStorage.getItem('DocumentFiltersForm_data');
    if (savedData) {
      this.data = JSON.parse(savedData);
      this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
    }
  }

  updateDataFromForm() {
    this.data = {};
    const formData = new FormData(this.firstElementChild);
    const formKeys = {
      'v-bpa:documentName_filter': String,
      'v-bpa:documentContent_filter': String,
      'v-s:created_filter': Array,
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
    sessionStorage.setItem('DocumentFiltersForm_data', JSON.stringify(this.data));
    this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
  }

  reset() {
    sessionStorage.removeItem('DocumentFiltersForm_data');
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
      <form on:submit="${(e) => this.submit(e)}" on:reset="${() => this.reset()}">
        <div class="mb-3">
          <label for="name" class="form-label" about="v-bpa:documentName" property="rdfs:label"></label>
          <input type="text" class="form-control" id="name" name="v-bpa:documentName_filter" value="${this.data['v-bpa:documentName_filter']?.[0] || ''}">
        </div>
        <div class="mb-3">
          <label for="content" class="form-label" about="v-bpa:documentContent" property="rdfs:label"></label>
          <input type="text" class="form-control" id="content" name="v-bpa:documentContent_filter" value="${this.data['v-bpa:documentContent_filter']?.[0] || ''}">
        </div>
        <div class="mb-3">
          <label class="form-label me-2" about="v-s:created" property="rdfs:label"></label>
          <div class="mb-3 d-flex align-items-center" id="created">
            <input type="date" placeholder="от" class="form-control me-2 w-50" name="v-s:created_filter" value="${this.data['v-s:created_filter']?.[0] || ''}">
            <input type="date" placeholder="до" class="form-control w-50" name="v-s:created_filter" value="${this.data['v-s:created_filter']?.[1] || ''}">
          </div>
        </div>
        <!--div class="mb-3 position-relative">
          <textarea class="form-control" rows="3" id="document-filter-raw-input" name="v-bpa:rawInput" on:change="${(e) => this.handleRawInput(e)}" value="${this.data['v-bpa:rawInput']?.[0] || ''}"></textarea>
          <div class="position-absolute" style="bottom: 0.125rem; right: 0.5rem;">
            <${InputAudio} for="document-filter-raw-input" id="document-filter-raw-input-audio"></${InputAudio}>
            <div class="d-none spinner-grow spinner-grow-sm" id="document-filter-raw-input-spinner"></div>
          </div>
        </div-->
        <button type="submit" class="btn btn-secondary me-2"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
        <button type="reset" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
      </form>
    `;
  }
}

customElements.define(DocumentFiltersForm.tag, DocumentFiltersForm);
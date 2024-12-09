import {Component, html, Model, timeout} from 'veda-client';
import InputAudio from './controls/InputAudio.js';

export default class ProcessFiltersForm extends Component(HTMLElement) {
  static tag = 'bpa-process-filters-form';

  data = {};

  added() {
    const savedData = sessionStorage.getItem('ProcessFiltersForm_data');
    if (savedData) {
      this.data = JSON.parse(savedData);
      this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
    }
  }

  updateDataFromForm() {
    this.data = {};
    const formData = new FormData(this.firstElementChild);
    const formKeys = {
      'rdfs:label_filter': String,
      'v-bpa:hasProcessJustification_filter': Array,
      'v-bpa:responsibleDepartment_filter': String,
      'v-bpa:laborCosts_filter': Array,
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
    sessionStorage.setItem('ProcessFiltersForm_data', JSON.stringify(this.data));
    this.dispatchEvent(new CustomEvent('apply', {detail: this.data}));
  }

  reset() {
    sessionStorage.removeItem('ProcessFiltersForm_data');
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
      <form on:submit="${(e) => this.submit(e)}" on:reset="${() => this.reset()}">
        <div class="mb-3">
          <label for="label" class="form-label" about="rdfs:label" property="rdfs:label"></label>
          <input type="text" class="form-control" id="label" name="rdfs:label_filter" value="${this.data['rdfs:label_filter']?.[0] || ''}">
        </div>
        <div class="mb-3">
          <label for="justification" class="form-label" about="v-bpa:hasProcessJustification" property="rdfs:label"></label>
          <select class="form-select" id="justification" name="v-bpa:hasProcessJustification_filter" multiple>
            <option value="v-bpa:CompletelyJustified" about="v-bpa:CompletelyJustified" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification_filter']?.includes('v-bpa:CompletelyJustified') ? 'selected' : ''}></option>
            <option value="v-bpa:PartlyJustified" about="v-bpa:PartlyJustified" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification_filter']?.includes('v-bpa:PartlyJustified') ? 'selected' : ''}></option>
            <option value="v-bpa:PoorlyJustified" about="v-bpa:PoorlyJustified" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification_filter']?.includes('v-bpa:PoorlyJustified') ? 'selected' : ''}></option>
            <option value="v-bpa:NoDocumentForJustification" about="v-bpa:NoDocumentForJustification" property="rdfs:label" ${this.data['v-bpa:hasProcessJustification_filter']?.includes('v-bpa:NoDocumentForJustification') ? 'selected' : ''}></option>
          </select>
        </div>
        <div class="mb-3">
          <label for="responsibleDepartment" class="form-label" about="v-bpa:responsibleDepartment" property="rdfs:comment"></label>
          <input type="text" class="form-control" id="responsibleDepartment" name="v-bpa:responsibleDepartment_filter" value="${this.data['v-bpa:responsibleDepartment_filter']?.[0] || ''}">
        </div>
        <div class="mb-3">
          <label class="form-label" for="laborCosts" about="v-bpa:laborCosts" property="rdfs:label"></label>
          <div class="mb-3 d-flex align-items-center" id="laborCosts">
            <input type="number" placeholder="от" class="form-control me-2 w-25" name="v-bpa:laborCosts_filter" value="${this.data['v-bpa:laborCosts_filter']?.[0] || ''}">
            <input type="number" placeholder="до" class="form-control w-25" name="v-bpa:laborCosts_filter" value="${this.data['v-bpa:laborCosts_filter']?.[1] || ''}">
          </div>
        </div>
        <div class="d-flex justify-content-between">
          <button type="submit" class="btn btn-secondary me-2"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
          <button type="reset" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
          <div class="mb-3 position-relative ms-auto">
            <input type="hidden" class="form-control" id="process-filter-raw-input" name="v-bpa:rawInput" on:change="${(e) => this.handleRawInput(e)}" value="${this.data['v-bpa:rawInput']?.[0] || ''}">
            <div class="position-absolute" style="bottom: -1rem; right: 0;">
              <${InputAudio} for="process-filter-raw-input" id="process-filter-raw-input-audio"></${InputAudio}>
              <div class="d-none spinner-grow spinner-grow-sm" id="process-filter-raw-input-spinner"></div>
            </div>
          </div>
        </div>
      </form>
    `;
  }
}

customElements.define(ProcessFiltersForm.tag, ProcessFiltersForm);
import {Component, html, Model, Backend} from 'veda-client';
import InputText from './controls/InputText.js';
import InputInteger from './controls/InputInteger.js';
import InputDecimal from './controls/InputDecimal.js';
import Textarea from './controls/Textarea.js';
import InputAudio from './controls/InputAudio.js';

export default class ProcessEdit extends Component(HTMLElement) {
  static tag = 'bpa-process-edit';

  async save() {
    await this.model.save();
    location.hash = `#/ProcessView/${this.model.id}`;
  }

  async cancel() {
    if (this.model.isNew()) {
      history.back();
    } else {
      await this.model.reset();
      location.hash = `#/ProcessView/${this.model.id}`;
    }
  }

  render() {
    return html`
      <div class="sheet">
        <h3 class="mb-1">
          <i class="bi bi-gear me-2"></i>
          <span about="v-bpa:BusinessProcess" property="rdfs:label"></span>
        </h3>
        <div class="row">
          <div class="col-12 col-md-8">
            <div class="mb-3">
              <label class="form-label fw-bold" about="rdfs:label" property="rdfs:label"></label>
              <input type="text" class="form-control" is="${InputText}" about="${this.model.id}" property="rdfs:label">
            </div>
            <div class="mb-3 position-relative">
              <label class="form-label fw-bold" about="v-bpa:processDescription" property="rdfs:label"></label>
              <textarea id="process-description" class="form-control" is="${Textarea}" about="${this.model.id}" property="v-bpa:processDescription" rows="10"></textarea>
              <div class="position-absolute bottom-0" style="right:0.75rem;">
                <${InputAudio} for="process-description"></${InputAudio}>
              </div>
            </div>

          </div>
          <div class="col-12 col-md-4 border-start border-secondary-subtle">
            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:responsibleDepartment" property="rdfs:label"></label>
              <input type="text" class="form-control" is="${InputText}" about="${this.model.id}" property="v-bpa:responsibleDepartment">
            </div>

            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:processParticipant" property="rdfs:label"></label>
              <textarea class="form-control" is="${Textarea}" about="${this.model.id}" property="v-bpa:processParticipant" rows="3"></textarea>
            </div>

            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:processFrequency" property="rdfs:label"></label>
              <div class="input-group">
                <input type="number" is="${InputInteger}" class="form-control" about="${this.model.id}" property="v-bpa:processFrequency">
                <span class="input-group-text" about="v-bpa:TimesPerYear" property="rdfs:label"></span>
              </div>
            </div>

            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:laborCosts" property="rdfs:label"></label>
              <div class="input-group">
                <input type="number" is="${InputDecimal}" class="form-control" about="${this.model.id}" property="v-bpa:laborCosts">
                <span class="input-group-text" about="v-bpa:Hours" property="rdfs:label"></span>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="sheet">
        <${ProcessDocumentAdd} about="${this.model.id}"></${ProcessDocumentAdd}>
      </div>
      <div class="d-flex justify-content-start gap-2 mt-3">
        <button on:click="${(e) => this.save(e)}" class="btn btn-success">
          <span about="v-bpa:Save" property="rdfs:label"></span>
        </button>
        <button on:click="${(e) => this.cancel(e)}" class="btn btn-link text-muted text-decoration-none">
          <span about="v-bpa:Cancel" property="rdfs:label"></span>
        </button>
      </div>
      `;
  }
}
customElements.define(ProcessEdit.tag, ProcessEdit);

class ProcessDocumentAdd extends Component(HTMLElement) {
  static tag = 'bpa-process-document-add';

  async getDocuments() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessDocuments';
    params['v-s:resultFormat'] = 'rows';
    const {rows: documents} = await Backend.stored_query(params);
    return documents;
  }

  async submit(e) {
    e.preventDefault();
    const form = e.target.closest('form');
    const selectedDocuments = Array.from(form.elements).filter(element => element.type === 'checkbox' && element.checked).map(element => element.value);
    this.model['v-bpa:hasProcessDocument'] = selectedDocuments.map(id => new Model(id));
  }

  async render() {
    return html`
      <h4 about="v-bpa:ChooseDocuments" property="rdfs:comment"></h4>
      <form id="process-document-add-form">
        ${(await this.getDocuments())?.map(([id]) => html`
          <div class="form-check d-flex gap-2 align-items-center">
            <input class="form-check-input mt-0" type="checkbox" value="${id}"
              ${this.model.hasValue('v-bpa:hasProcessDocument', id) ? 'checked' : ''}
              on:change="${(e) => this.submit(e)}">
            <label class="form-check-label w-100">
              <div class="card mb-1 bg-light border-light" about="${id}">
                <div class="card-body p-2">
                  <div class="card-title mb-0">
                    <a class="text-dark text-decoration-none d-flex align-items-center" href="#/DocumentView/${id}">
                      <i class="fs-4 bi bi-file-earmark-text me-2"></i>
                      <span property="v-bpa:documentName"></span>
                      <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString('ru-RU') }}</span>
                    </a>
                  </div>
                </div>
              </div>
            </label>
          </div>
        `).join('')}
      </form>
    `;
  }
}
customElements.define(ProcessDocumentAdd.tag, ProcessDocumentAdd);

import {Component, html, Model, Backend} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Expression from './Expression.js';
import * as Bootstrap from 'bootstrap';

export default class ProcessDocumentInfo extends Component(HTMLElement) {
  static tag = 'bpa-process-document-info';

  async render() {
    return html`
      <a href="#process-document-list-modal" data-bs-toggle="modal" data-bs-target="#process-document-list-modal" class="text-secondary" style="cursor:pointer">
        <span about="v-bpa:ProcessDocuments" property="rdfs:label"></span>:&nbsp;<${Expression} about="${this.model.id}" expression="this['v-bpa:hasProcessDocument']?.length || '+'"></${Expression}>
      </a>
      <div class="modal fade" id="process-document-list-modal">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <${ProcessDocumentList} about="${this.model.id}"></${ProcessDocumentList}>
            </div>
          </div>
        </div>
      </div>
      <div class="modal fade" id="process-document-add-modal">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <${ProcessDocumentAdd} about="${this.model.id}"></${ProcessDocumentAdd}>
            </div>
          </div>
        </div>
      </div>
      <div class="modal fade" id="process-document-preview-modal">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <${DocumentPreview} id="process-document-preview"></${DocumentPreview}>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}
customElements.define(ProcessDocumentInfo.tag, ProcessDocumentInfo);

class ProcessDocumentList extends Component(HTMLElement) {
  static tag = 'bpa-process-document-list';

  previewDocument(e, model) {
    e.preventDefault();
    const preview = document.getElementById('process-document-preview');
    preview.setAttribute('about', model.id);
    preview.setAttribute('back', 'process-document-list-modal');
  }

  render() {
    return html`
      <div class="d-flex justify-content-between">
        <div class="fs-5 mb-2" about="${this.model.id}" rel="v-bpa:hasProcessJustification">
          <${ProcessJustificationIndicator} about="{{this.model.id}}" property="rdfs:comment"></${ProcessJustificationIndicator}>
        </div>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <div property="v-bpa:justificationReason" class="mb-3"></div>
      <div rel="v-bpa:hasProcessDocument">
        <div class="card mb-3 bg-light border-light">
          <div class="card-body p-2">
            <div class="card-title mb-0">
              <a class="text-dark text-decoration-none d-flex align-items-center" href="#process-document-preview-modal" data-bs-toggle="modal" data-bs-target="#process-document-preview-modal" @click="${(e) => this.parent.previewDocument(e, this.model)}">
                <i class="fs-4 bi bi-file-earmark-text me-2"></i>
                <span property="v-bpa:documentName"></span>
                <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString() }}</span>
              </a>
            </div>
          </div>
        </div>
      </div>
      <div class="text-end">
        <button type="button" class="btn btn-primary" data-bs-toggle="modal" data-bs-target="#process-document-add-modal">
          <span about="v-bpa:ChooseDocuments" property="rdfs:label"></span>
        </button>
      </div>
    `;
  }
}
customElements.define(ProcessDocumentList.tag, ProcessDocumentList);

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

  async submit(event) {
    event.preventDefault();
    const form = event.target;
    const selectedDocuments = Array.from(form.elements).filter(element => element.type === 'checkbox' && element.checked).map(element => element.value);
    this.model['v-bpa:hasProcessDocument'] = selectedDocuments.map(id => new Model(id));
    await this.model.save();
  }

  previewDocument(e, model) {
    e.preventDefault();
    const preview = document.getElementById('process-document-preview');
    preview.setAttribute('about', model.id);
    preview.setAttribute('back', 'process-document-add-modal');
  }

  async render() {
    return html`
      <div class="d-flex justify-content-between">
        <h4 about="v-bpa:ChooseDocuments" property="rdfs:comment"></h4>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <form id="process-document-add-form" @submit="${(e) => this.submit(e)}">
        ${(await this.getDocuments())?.map(([id]) => html`
          <div class="form-check d-flex gap-2 align-items-center">
            <input class="form-check-input mt-0" type="checkbox" value="${id}" ${this.model.hasValue('v-bpa:hasProcessDocument', id) ? 'checked' : ''}>
            <label class="form-check-label w-100">
              <div class="card mb-1 bg-light border-light" about="${id}">
                <div class="card-body p-2">
                  <div class="card-title mb-0">
                    <a class="text-dark text-decoration-none d-flex align-items-center" href="#process-document-preview-modal" data-bs-toggle="modal" data-bs-target="#process-document-preview-modal" style="cursor:pointer" @click="${(e) => this.parent.previewDocument(e, this.model)}">
                      <i class="fs-4 bi bi-file-earmark-text me-2"></i>
                      <span property="v-bpa:documentName"></span>
                      <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString() }}</span>
                    </a>
                  </div>
                </div>
              </div>
            </label>
          </div>
        `).join('')}
        <div class="d-flex justify-content-between mt-2">
          <button type="button" class="btn btn-light" data-bs-toggle="modal" data-bs-target="#process-document-list-modal">
            <i class="bi bi-arrow-left me-1"></i>
            <span about="v-bpa:Back" property="rdfs:label"></span>
          </button>
          <button type="submit" class="btn btn-primary" data-bs-toggle="modal" data-bs-target="#process-document-list-modal">
            <span about="v-bpa:ChooseDocuments" property="rdfs:label"></span>
          </button>
        </div>
      </form>
    `;
  }
}
customElements.define(ProcessDocumentAdd.tag, ProcessDocumentAdd);

class DocumentPreview extends Component(HTMLElement) {
  static tag = 'bpa-document-preview';

  static get observedAttributes() {
    return ['about', 'back'];
  }

  async attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'about') {
      this.model = await new Model(newValue).load();
    }
    await this.update();
  }

  render() {
    if (!this.model) {
      return '';
    }
    return html`
      <div class="d-flex justify-content-between mb-2">
        <h4 class="mb-0">
          <i class="bi bi-file-earmark-text me-2"></i>
          <span class="me-3" property="v-bpa:documentName"></span>
        </h4>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <p property="v-bpa:documentContent"></p>
      <button type="button" class="btn btn-light" data-bs-toggle="modal" data-bs-target="#${this.getAttribute('back')}">
        <i class="bi bi-arrow-left me-1"></i>
        <span about="v-bpa:Back" property="rdfs:label"></span>
      </button>
    `;
  }
}
customElements.define(DocumentPreview.tag, DocumentPreview);

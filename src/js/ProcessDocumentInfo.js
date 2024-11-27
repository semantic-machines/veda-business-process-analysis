import {Component, html, safe, Model, Backend} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Expression from './Expression.js';
import {Modal} from 'bootstrap';

export default class ProcessDocumentInfo extends Component(HTMLElement) {
  static tag = 'bpa-process-document-info';

  async render() {
    return html`
      <a href="#process-document-list-modal" data-bs-toggle="modal" data-bs-target="#process-document-list-modal" class="text-secondary" style="cursor:pointer">
        <span about="v-bpa:ProcessDocuments" property="rdfs:label"></span>:&nbsp;<${Expression} about="${this.model.id}" expression="this['v-bpa:hasProcessDocument']?.length || '+'"></${Expression}>
      </a>
      <${ProcessDocumentListModal} about="${this.model.id}"></${ProcessDocumentListModal}>
      <${ProcessDocumentAddModal} about="${this.model.id}"></${ProcessDocumentAddModal}>
      <${ProcessDocumentPreviewModal} id="process-document-preview"></${ProcessDocumentPreviewModal}>
    `;
  }

  removed() {
    const modals = this.querySelectorAll('.modal');
    modals.forEach(modal => Modal.getInstance(modal)?.hide());
  }
}
customElements.define(ProcessDocumentInfo.tag, ProcessDocumentInfo);

class ProcessDocumentListModal extends Component(HTMLElement) {
  static tag = 'bpa-process-document-list-modal';

  previewDocument(e, model) {
    e.preventDefault();
    const preview = document.getElementById('process-document-preview');
    preview.setAttribute('about', model.id);
    preview.setAttribute('back', 'process-document-list-modal');
  }

  post() {
    this.firstElementChild.addEventListener('shown.bs.modal', () => {
      this.querySelector('.btn-close')?.focus();
    });
  }

  render() {
    return html`
      <div class="modal fade" id="process-document-list-modal" data-bs-keyboard="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
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
                        <span class="me-2" property="v-bpa:documentName"></span>
                        <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString('ru-RU') }}</span>
                      </a>
                    </div>
                  </div>
                </div>
              </div>
              <div class="d-flex justify-content-between">
                <button type="button" class="btn btn-light" data-bs-dismiss="modal">
                  <span about="v-bpa:Close" property="rdfs:label"></span>
                </button>
                <button type="button" class="btn btn-primary" data-bs-toggle="modal" data-bs-target="#process-document-add-modal">
                  <span about="v-bpa:ChangeDocuments" property="rdfs:label"></span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}
customElements.define(ProcessDocumentListModal.tag, ProcessDocumentListModal);

class ProcessDocumentAddModal extends Component(HTMLElement) {
  static tag = 'bpa-process-document-add-modal';

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

  async cancel(event) {
    // event.preventDefault();
    this.model.reset();
  }

  previewDocument(e, model) {
    e.preventDefault();
    const preview = document.getElementById('process-document-preview');
    preview.setAttribute('about', model.id);
    preview.setAttribute('back', 'process-document-add-modal');
  }

  post() {
    this.firstElementChild.addEventListener('shown.bs.modal', () => {
      this.querySelector('.btn-back')?.focus();
    });
  }

  async render() {
    return html`
      <div class="modal fade" id="process-document-add-modal" data-bs-keyboard="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <div class="d-flex justify-content-between">
                <h4 about="v-bpa:ChooseDocuments" property="rdfs:comment"></h4>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
              </div>
              <form id="process-document-add-form" @submit="${(e) => this.submit(e)}" @reset="${(e) => this.cancel(e)}">
                ${(await this.getDocuments())?.map(([id]) => html`
                  <div class="form-check d-flex gap-2 align-items-center">
                    <input class="form-check-input mt-0" type="checkbox" value="${id}" ${this.model.hasValue('v-bpa:hasProcessDocument', id) ? 'checked' : ''} style="cursor:pointer">
                    <label class="form-check-label w-100">
                      <div class="card mb-1 bg-light border-light" about="${id}">
                        <div class="card-body p-2">
                          <div class="card-title mb-0">
                            <a class="text-dark text-decoration-none d-flex align-items-center" href="#process-document-preview-modal" data-bs-toggle="modal" data-bs-target="#process-document-preview-modal" style="cursor:pointer" @click="${(e) => this.parent.previewDocument(e, this.model)}">
                              <i class="fs-4 bi bi-file-earmark-text me-2"></i>
                              <span class="me-2" property="v-bpa:documentName"></span>
                              <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString('ru-RU') }}</span>
                            </a>
                          </div>
                        </div>
                      </div>
                    </label>
                  </div>
                `).join('')}
                <div class="d-flex justify-content-between mt-2">
                  <button type="reset" class="btn btn-light btn-back" data-bs-toggle="modal" data-bs-target="#process-document-list-modal">
                    <span about="v-bpa:Cancel" property="rdfs:label"></span>
                  </button>
                  <button type="submit" class="btn btn-primary" data-bs-toggle="modal" data-bs-target="#process-document-list-modal">
                    <span about="v-bpa:ChooseDocuments" property="rdfs:label"></span>
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}
customElements.define(ProcessDocumentAddModal.tag, ProcessDocumentAddModal);

class ProcessDocumentPreviewModal extends Component(HTMLElement) {
  static tag = 'bpa-process-document-preview-modal';

  static get observedAttributes() {
    return ['about', 'back'];
  }

  async attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'about') {
      this.renderDocumentPreview();
    }
    if (name === 'back') {
      this.querySelector('button').setAttribute('data-bs-target', `#${newValue}`);
    }
  }

  renderDocumentPreview() {
    const container = document.getElementById('process-document-preview-container');
    const preview = document.createElement(`${ProcessDocumentPreview}`);
    preview.setAttribute('about', this.getAttribute('about'));
    preview.setAttribute('back', this.getAttribute('back'));
    container.replaceChildren(preview);
  }

  post() {
    this.firstElementChild.addEventListener('shown.bs.modal', () => {
      this.querySelector('.btn-back')?.focus();
    });
  }

  render() {
    return html`
      <div class="modal fade" id="process-document-preview-modal" data-bs-keyboard="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <div id="process-document-preview-container"></div>
              <button type="button" class="btn btn-light btn-back" data-bs-toggle="modal">
                <i class="bi bi-arrow-left me-1"></i>
                <span about="v-bpa:Back" property="rdfs:label"></span>
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}
customElements.define(ProcessDocumentPreviewModal.tag, ProcessDocumentPreviewModal);

class ProcessDocumentPreview extends Component(HTMLElement) {
  static tag = 'bpa-process-document-preview';

  render() {
    return html`
      <div class="d-flex justify-content-between mb-2">
        <h4 class="mb-0">
          <i class="bi bi-file-earmark-text me-2"></i>
          <span class="me-3" property="v-bpa:documentName"></span>
        </h4>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <p property="v-bpa:documentContent"></p>
    `;
  }
}
customElements.define(ProcessDocumentPreview.tag, ProcessDocumentPreview);
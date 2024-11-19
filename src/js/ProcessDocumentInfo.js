import {Component, html, Model, Backend} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';

export default class ProcessDocumentInfo extends Component(HTMLElement) {
  static tag = 'bpa-process-document-info';

  async render() {
    const documentsCount = this.model['v-bpa:hasProcessDocument']?.length;
    return html`
      ${documentsCount
        ? html`
            <a href="#process-document-modal" data-bs-toggle="modal" data-bs-target="#process-document-modal" class="text-secondary" style="cursor:pointer">
              <span about="v-bpa:ProcessDocuments" property="rdfs:label"></span>: ${documentsCount}
            </a>
            <div class="modal fade" id="process-document-modal">
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
          `
        : html`
            <a class="text-secondary" data-bs-toggle="modal" data-bs-target="#process-document-add-modal">
              <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
            </a>
          `
      }
    `;
  }
}

customElements.define(ProcessDocumentInfo.tag, ProcessDocumentInfo);

class ProcessDocumentList extends Component(HTMLElement) {
  static tag = 'bpa-process-document-list';

  render() {
    return html`
      <div class="d-flex justify-content-between">
        <div class="fs-5 mb-2" about="${this.model.id}" rel="v-bpa:hasProcessJustification">
          <${ProcessJustificationIndicator} about="{{this.model.id}}" property="rdfs:comment"></${ProcessJustificationIndicator}>
        </div>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <div property="v-bpa:justificationReason" class="mb-3"></div>
      <h5 about="v-bpa:ProcessDocuments" property="rdfs:label"></h5>
      <div rel="v-bpa:hasProcessDocument">
        <div class="card mb-3 bg-light border-light">
          <div class="card-body p-2">
            <div class="card-title mb-0 d-flex align-items-center">
              <i class="fs-4 bi bi-file-earmark-text text-secondary me-2"></i>
              <span property="v-bpa:documentName"></span>
              <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString() }}</span>
            </div>
          </div>
        </div>
      </div>
      <button type="button" class="btn btn-light" data-bs-toggle="modal" data-bs-target="#process-document-add-modal">
        <i class="bi bi-plus me-1"></i>
        <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
      </button>
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
    console.log(selectedDocuments);
    this.model['v-bpa:hasProcessDocument'] = selectedDocuments.map(id => new Model(id));
    await this.model.save();
  }

  async render() {
    return html`
      <div class="d-flex justify-content-between">
        <h4 about="v-bpa:ChooseDocuments" property="rdfs:label"></h4>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <form id="process-document-add-form" @submit="submit">
        ${(await this.getDocuments())?.map(([id]) => html`
          <div class="form-check d-flex gap-2 align-items-center">
            <input class="form-check-input mt-0" type="checkbox" value="${id}" ${this.model.hasValue('v-bpa:hasProcessDocument', id) ? 'checked' : ''}>
            <label class="form-check-label w-100">
              <div class="card mb-1 bg-light border-light" about="${id}">
                <div class="card-body p-2">
                  <div class="card-title mb-0 d-flex align-items-center">
                    <i class="fs-4 bi bi-file-earmark-text text-secondary me-2"></i>
                    <span property="v-bpa:documentName"></span>
                    <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString() }}</span>
                  </div>
                </div>
              </div>
            </label>
          </div>
        `).join('')}
        <button type="submit" class="btn btn-primary" data-bs-toggle="modal" data-bs-target="#process-document-modal">
          <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
        </button>
      </form>
    `;
  }
}

customElements.define(ProcessDocumentAdd.tag, ProcessDocumentAdd);

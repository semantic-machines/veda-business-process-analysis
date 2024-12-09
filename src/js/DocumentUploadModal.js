import {Component, html, Model, genUri, guid, decorator, Backend} from 'veda-client';
import {Modal} from 'bootstrap';
import spinnerDecorator from './Spinner';

export default class DocumentUploadModal extends Component(HTMLElement) {
  static tag = 'bpa-document-upload-modal';


  hideModal() {
    Modal.getOrCreateInstance(this.firstElementChild).hide();
  }

  post() {
    this.firstElementChild.addEventListener('shown.bs.modal', () => {
      this.querySelector('button')?.focus();
    });
  }

  render() {
    return html`
      <div class="modal fade" id="document-upload-modal" data-bs-keyboard="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <${DocumentUpload}
                on:uploaded="${(e) => this.hideModal()}"
                on:canceled="${(e) => this.hideModal()}">
              </${DocumentUpload}>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}
customElements.define(DocumentUploadModal.tag, DocumentUploadModal);

class DocumentUpload extends Component(HTMLElement) {
  static tag = 'bpa-document-upload';

  selectedFiles = new Map;
  uploadedFiles = new Map;

  handleDragOver(e) {
    e.preventDefault();
    e.currentTarget.classList.add('drag-over');
  }

  handleDragLeave(e) {
    e.currentTarget.classList.remove('drag-over');
  }

  handleDrop(e) {
    e.preventDefault();
    e.currentTarget.classList.remove('drag-over');
    const files = e.dataTransfer.files;
    if (files.length) {
      Array.from(files).forEach(file => {
        this.selectedFiles.set(file.name, file);
      });
      this.update();
    }
  }

  handleFileSelect(e) {
    const files = e.target.files;
    if (files.length) {
      Array.from(files).forEach(file => {
        this.selectedFiles.set(file.name, file);
      });
      this.update();
    }
  }

  removeFile(fileName) {
    this.selectedFiles.delete(fileName);
    this.update();
  }

  upload = spinnerDecorator(async () => {
    const files = Array.from(this.selectedFiles.values());
    for (const file of files) {
      if (this.uploadedFiles.has(file.name)) continue;
      const fileIndividual = await this.createFileIndividual(file);
      this.uploadedFiles.set(file.name, fileIndividual);
      this.update();
      await this.createProcessDocumentRequest(fileIndividual);
    }
  });

  async createFileIndividual(file) {
    const path = new Date().toLocaleDateString('ru-RU').split('.').reverse().join('/');
    const uri = guid();
    const fileIndividual = new Model;
    fileIndividual['rdf:type'] = 'v-s:File';
    fileIndividual['rdfs:label'] = file.name;
    fileIndividual['v-s:fileName'] = file.name;
    fileIndividual['v-s:fileSize'] = file.size;
    fileIndividual['v-s:fileUri'] = uri;
    fileIndividual['v-s:filePath'] = path;
    try {
      await Backend.uploadFile({file, uri, path});
      return fileIndividual.save();
    } catch (error) {
      console.error(`Ошибка при загрузке файла '${file.name}': ${error.message}`, error);
      alert(`Ошибка при загрузке файла '${file.name}': ${error.message}`);
    }
  }

  async createProcessDocumentRequest(fileIndividual) {
    const request = new Model;
    request['rdf:type'] = 'v-bpa:GenericProcessingRequest';
    request['v-bpa:prompt'] = 'v-bpa:DocumentAnalysisPrompt';
    request['v-s:attachment'] = fileIndividual;
    return request.save();
  }

  cancel() {
    this.selectedFiles.clear();
    this.uploadedFiles.clear();
    this.dispatchEvent(new CustomEvent('canceled'));
    this.update();
  }

  removed() {
    this.selectedFiles.clear();
    this.uploadedFiles.clear();
  }

  render() {
    return html`
      <div>
        <div class="d-flex justify-content-between">
          <h3 class="mb-1">
            <i class="bi bi-file-earmark-text me-2"></i>
            <span about="v-bpa:DocumentUpload" property="rdfs:label"></span>
          </h3>
          <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
        </div>
        <p class="text-muted fw-bold" about="v-bpa:DocumentUpload" property="rdfs:comment"></p>
        <div class="mb-3">
          <div class="drop-zone p-4 border rounded text-center"
            on:dragover="${(e) => this.handleDragOver(e)}"
            on:dragleave="${(e) => this.handleDragLeave(e)}"
            on:drop="${(e) => this.handleDrop(e)}">
            ${this.selectedFiles.size === 0
              ? html`
                <i class="bi bi-cloud-upload display-4"></i>
                <p class="mb-2" about="v-bpa:DragAndDrop" property="rdfs:label"></p>
                <input type="file" multiple class="d-none" on:change="${(e) => this.handleFileSelect(e)}" id="file-input">
                <label for="file-input" class="btn btn-outline-primary" about="v-bpa:ChooseFiles" property="rdfs:label"></label>`
              : html`
                <div class="selected-files mb-4">
                  ${Array.from(this.selectedFiles.keys()).map(fileName => html`
                    <div class="selected-file d-flex align-items-center justify-content-between p-2 border rounded mb-2 bg-white">
                      <div class="text-start">
                        <i class="bi bi-file-earmark-text"></i>
                        <span>${fileName}</span>
                      </div>
                      ${this.uploadedFiles.has(fileName)
                        ? html`<span class="bi bi-check-circle-fill text-success"></span>`
                        : html`<button type="button" class="btn-close" on:click="() => this.removeFile('${fileName}')"></button>`}
                    </div>`
                  ).join('')}
                </div>
                <input type="file" multiple class="d-none" on:change="${(e) => this.handleFileSelect(e)}" id="file-input">
                <label for="file-input" class="btn btn-outline-primary">Добавить ещё файлы</label>
              `}
          </div>
        </div>
        <div class="d-flex justify-content-between gap-2">
          <button on:click="${(e) => this.cancel(e)}" class="btn btn-light">
            <span about="v-bpa:Close" property="rdfs:label"></span>
          </button>
          <button on:click="${(e) => this.upload(e)}" class="btn btn-primary create-button">
            <span class="spinner-grow spinner-grow-sm me-2 d-none" aria-hidden="true"></span>
            <span about="v-bpa:Upload" property="rdfs:label"></span>
          </button>
        </div>
        <style>
          .drop-zone {
            transition: all 0.2s ease;
            background: #f8f9fa;
          }
          .drop-zone.drag-over {
            background: #e9ecef;
            border-style: dashed !important;
          }
        </style>
      </div>
    `;
  }
}
customElements.define(DocumentUpload.tag, DocumentUpload);

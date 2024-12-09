import {Component, html, safe, Backend, Model} from 'veda-client';
import DocumentFiltersModal from './DocumentFiltersModal.js';
import DocumentUploadModal from './DocumentUploadModal.js';

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
        if (this.filtersData['v-bpa:documentName_filter'] && this.filtersData['v-bpa:documentName_filter'][0] &&
            !name.toLowerCase().includes(this.filtersData['v-bpa:documentName_filter'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по содержанию
        if (this.filtersData['v-bpa:documentContent_filter'] && this.filtersData['v-bpa:documentContent_filter'][0] &&
            !content.toLowerCase().includes(this.filtersData['v-bpa:documentContent_filter'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по дате создания
        created = new Date(created);
        if (this.filtersData['v-s:created_filter'] && this.filtersData['v-s:created_filter'][0]) {
          const from = new Date(this.filtersData['v-s:created_filter'][0]);
          from.setHours(0, 0, 0, 0);
          if (created < from) return false;
        }
        if (this.filtersData['v-s:created_filter'] && this.filtersData['v-s:created_filter'][1]) {
          const to = new Date(this.filtersData['v-s:created_filter'][1]);
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
    this.filtered.forEach(([...values]) => {
      const [id, name, content, created] = safe(values);
      const row = document.createElement('tr');
      row.onclick = () => location.hash = `#/DocumentView/${id}`;
      row.innerHTML = `
        <td class="align-middle"><h5 class="mb-0">${name}</h5><p class="text-muted mb-0">${content && content.length > 120 ? content.slice(0, 120) + '...' : content}</p></td>
        <td class="align-middle text-end">${new Date(created).toLocaleDateString('ru-RU')}</td>
      `;
      fragment.appendChild(row);
    });
    container.innerHTML = '';
    container.appendChild(fragment);
  }

  post() {
    this.renderFilteredDocuments();
    this.querySelector(`${DocumentFiltersModal}`).addEventListener('filters-changed', this.handleFiltersChange);
  }

  render() {
    return html`
      <div class="sheet">
        <div class="d-flex align-items-center">
          <i class="bi bi-file-earmark-text ms-2 me-3 fs-1"></i>
          <h3 class="mb-1" about="v-bpa:ProcessDocuments" property="rdfs:label"></h3>
          <div class="d-flex align-items-center ms-auto">
            <a href="#document-upload-modal" data-bs-toggle="modal" data-bs-target="#document-upload-modal" class="btn btn-link text-dark text-decoration-none me-3">
              <i class="bi bi-plus me-1"></i>
              <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
            </a>
            <${DocumentUploadModal}></${DocumentUploadModal}>
            <${DocumentFiltersModal}></${DocumentFiltersModal}>
          </div>
        </div>
        <div class="table-responsive">
          <style>
            #documents-table tbody tr:last-child {
              border-bottom: 1px solid transparent;
            }
            #documents-table tr {
              cursor: pointer;
            }
          </style>
          <table class="table table-hover mb-0" id="documents-table">
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

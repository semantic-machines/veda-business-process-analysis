import {Component, html, safe, Backend, Model} from 'veda-client';
import DocumentFiltersModal from './DocumentFiltersModal.js';
import DocumentUploadModal from './DocumentUploadModal.js';
import Literal from './Literal.js';

export default class DocumentList extends Component(HTMLElement) {
  static tag = 'bpa-document-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessDocuments';
    params['v-s:resultFormat'] = 'rows';
    try {
      const {rows: documents} = await Backend.stored_query(params);
      this.documents = documents;
    } catch (e) {
      console.error('Error querying documents', e);
      this.documents = [];
    }
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
      this.filtered = this.documents.filter(([id, title, type, department, created]) => {
        // Фильтр по названию документа
        if (this.filtersData['v-bpa:documentTitle_filter'] && this.filtersData['v-bpa:documentTitle_filter'][0] &&
            !title.toLowerCase().includes(this.filtersData['v-bpa:documentTitle_filter'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по типу документа
        if (this.filtersData['v-bpa:documentType_filter'] && this.filtersData['v-bpa:documentType_filter'][0] &&
            !type.toLowerCase().includes(this.filtersData['v-bpa:documentType_filter'][0].toLowerCase())) {
          return false;
        }
        // Фильтр по отделу
        if (this.filtersData['v-bpa:hasDepartment_filter'] && this.filtersData['v-bpa:hasDepartment_filter'][0] &&
            !department.toLowerCase().includes(this.filtersData['v-bpa:hasDepartment_filter'][0].toLowerCase())) {
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
      const [id, title, type, department, created] = safe(values);
      const row = document.createElement('tr');
      row.onclick = () => location.hash = `#/DocumentView/${id}`;
      row.setAttribute('about', id);
      row.innerHTML = `
        <td class="align-middle"><h5 class="mb-0">${title}</h5></td>
        <td class="align-middle">${type}</td>
        <td class="align-middle"><${Literal} about="${department}" property="rdfs:label"></${Literal}></td>
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

  removed () {
    this.querySelector(`${DocumentFiltersModal}`).removeEventListener('filters-changed', this.handleFiltersChange);
  }

  render() {
    return html`
      <div class="sheet">
        <div class="d-flex align-items-center">
          <i class="bi bi-file-earmark-text ms-2 me-3 fs-1"></i>
          <h3 class="mb-1" about="v-bpa:ProcessDocuments" property="rdfs:label"></h3>
          <div class="d-flex align-items-center ms-auto">
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
                <th width="55%" class="text-secondary fw-normal" about="v-bpa:documentTitle" property="rdfs:label"></th>
                <th width="15%" class="text-secondary fw-normal text-nowrap" about="v-bpa:documentType" property="rdfs:label"></th>
                <th width="15%" class="text-secondary fw-normal text-nowrap" about="v-bpa:hasDepartment" property="rdfs:label"></th>
                <th width="15%" class="text-secondary fw-normal text-end text-nowrap" about="v-s:created" property="rdfs:label"></th>
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

import {Component, html, safe, Backend, Model} from 'veda-client';
import DocumentFiltersModal from './DocumentFiltersModal.js';
import DocumentUploadModal from './DocumentUploadModal.js';
import Literal from './Literal.js';
import state from './State.js';

function zip(a, b) {
  return a.map((_, i) => [a[i], b[i]]);
}

export default class DocumentList extends Component(HTMLElement) {
  static tag = 'bpa-document-list';

  async added() {
    state.on('document-processing-pipeline-completed', this.onPipelineCompleted);
  }

  onPipelineCompleted = () => this.update();

  async getDocuments() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessDocuments';
    params['v-s:resultFormat'] = 'rows';
    try {
      const {rows: documents} = await Backend.stored_query(params);
      return documents;
    } catch (e) {
      console.error('Error querying documents', e);
      return [];
    }
  }

  async getRunningProcessExtractionPipelines() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:RunningProcessExtractionPipelines';
    params['v-s:resultFormat'] = 'cols';
    try {
      const {id: pipelines, "v-bpa:targetDepartment": departments} = await Backend.stored_query(params);
      return new Map(zip(departments, pipelines));
    } catch (e) {
      console.error('Error querying running process extraction pipelines', e);
      return new Map();
    }
  }

  async pre() {
    this.documents = await this.getDocuments();
    this.runningProcessExtractionPipelinesByDepartment = await this.getRunningProcessExtractionPipelines();
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
      this.filtered = this.documents.filter(([id, title, type, department, departmentLabel, created, statusTag, statusTagColor]) => {
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
            !departmentLabel.toLowerCase().includes(this.filtersData['v-bpa:hasDepartment_filter'][0].toLowerCase())) {
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

    // Группируем документы по подразделениям
    const departments = {};
    this.filtered.forEach(values => {
      const [id, title, type, department, departmentLabel, created, statusTag, statusTagColor] = safe(values);
      if (!departments[department]) {
        departments[department] = {
          label: departmentLabel || 'Без отдела',
          hasChanges: false,
          docs: []
        };
      }
      departments[department].hasChanges = departments[department].hasChanges || !!statusTag;
      departments[department].docs.push({id, title, type, departmentLabel, created, statusTag, statusTagColor});
    });

    // Рендерим по подразделениям
    Object.entries(departments).forEach(([department, {label, hasChanges, docs}]) => {
      // Заголовок подразделения
      const departmentRow = document.createElement('tr');
      departmentRow.setAttribute('about', department);
      departmentRow.className = 'table-light';
      departmentRow.innerHTML = `
        <td colspan="4" class="text-uppercase text-secondary rounded-bottom">
          <div class="d-flex align-items-center justify-content-between">
            <small>${label}</small>
            <button class="btn btn-sm btn-outline-secondary-light apply-changes d-none">Применить изменения</button>
          </div>
        </td>
      `;
      if (hasChanges) {
        const applyChangesButton = departmentRow.querySelector('.apply-changes');
        applyChangesButton.classList.remove('d-none');
        applyChangesButton.addEventListener('click', () => this.applyChanges(department));
      }
      fragment.appendChild(departmentRow);

      // Документы подразделения
      docs.forEach(doc => {
        const row = document.createElement('tr');
        row.onclick = () => location.hash = `#/DocumentView/${doc.id}`;
        row.setAttribute('about', doc.id);
        row.innerHTML = `
          <td class="align-middle"><strong class="me-1">${doc.title}</strong> <small class="badge bg-${doc.statusTagColor}-subtle text-${doc.statusTagColor}">${doc.statusTag}</small></td>
          <td class="align-middle">${doc.type}</td>
          <td class="align-middle">${doc.departmentLabel}</td>
          <td class="align-middle text-end">${new Date(doc.created).toLocaleDateString('ru-RU')}</td>
        `;
        fragment.appendChild(row);
      });
    });

    container.innerHTML = '';
    container.appendChild(fragment);
  }

  applyChanges = (department) => {
    try {
      const businessProcessExtractionPipeline = new Model;
      businessProcessExtractionPipeline['rdf:type'] = 'v-bpa:PipelineRequest';
      businessProcessExtractionPipeline['v-bpa:pipeline'] = 'v-bpa:businessProcessExtractionPipeline';
      businessProcessExtractionPipeline['v-bpa:targetDepartment'] = department;
      businessProcessExtractionPipeline.save();
    } catch (e) {
      console.error('Ошибка запуска процедуры вычисления процессов', e);
      alert('Ошибка запуска процедуры вычисления процессов');
    }
  }

  post() {
    this.renderFilteredDocuments();
    this.querySelector(`${DocumentFiltersModal}`).addEventListener('filters-changed', this.handleFiltersChange);
  }

  removed () {
    this.querySelector(`${DocumentFiltersModal}`).removeEventListener('filters-changed', this.handleFiltersChange);
    state.off('document-processing-pipeline-completed', this.onPipelineCompleted);
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
            #documents-table td {
              cursor: pointer;
            }
            #documents-table tr.table-light > td {
              background-color: #f2f2f2;
              cursor: default;
              border-bottom: 1px solid transparent;
            }
            #documents-table tr.table-light:hover > td {
              box-shadow: none;
            }
          </style>
          <table class="table table-hover mb-0" id="documents-table">
            <thead>
              <tr>
                <th width="45%" class="text-secondary fw-normal" about="v-bpa:documentTitle" property="rdfs:label"></th>
                <th width="15%" class="text-secondary fw-normal text-nowrap" about="v-bpa:documentType" property="rdfs:label"></th>
                <th width="30%" class="text-secondary fw-normal text-nowrap" about="v-bpa:hasDepartment" property="rdfs:label"></th>
                <th width="10%" class="text-secondary fw-normal text-end text-nowrap" about="v-s:created" property="rdfs:label"></th>
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

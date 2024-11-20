import {Component, html, Model, Backend, timeout} from 'veda-client';
import ProcessList from './ProcessList.js';
import DocumentList from './DocumentList.js';
import ClusterList from './ClusterList.js';
import ClusterizationButton from './ClusterizationButton.js';
import Callback from './Callback.js';
import ProcessQuickCreate from './ProcessQuickCreate.js';

export default class ProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  activeTab = localStorage.getItem('ProcessOverview_activeTab') || 'processes';
  
  async added() {
    const params1 = new Model;
    params1['rdf:type'] = 'v-s:QueryParams';
    params1['v-s:storedQuery'] = 'v-bpa:OverallCounts';
    params1['v-s:resultFormat'] = 'cols';
    try {
      const {processes: [processesCount], documents: [documentsCount]} = await Backend.stored_query(params1);
      this.processesCount = processesCount;
      this.documentsCount = documentsCount;
    } catch (e) {
      console.error('Error querying overall counts', e);
    }

    const params2 = new Model;
    params2['rdf:type'] = 'v-s:QueryParams';
    params2['v-s:storedQuery'] = 'v-bpa:CompletedAndRunningClusterizationAttempts';
    params2['v-s:resultFormat'] = 'cols';
    try {
      const {completed: [completed], running: [running]} = await Backend.stored_query(params2);
      this.completed = completed ? await new Model(completed).load() : null;
      this.running = running ? await new Model(running).load() : null;
      this.running?.on('modified', this.switchClusterList);
      Callback.set('ProcessOverview_setRunning', this.setRunning);
    } catch (e) {
      console.error('Error querying running clusterization attempts', e);
    }
  }

  setRunning = (running) => {
    this.running = running;
    this.running?.on('modified', this.switchClusterList);
  }

  switchClusterList = () => {
    if (this.running.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionCompleted')) {
      this.completed = this.running;
      this.running.off('modified', this.switchClusterList);
      this.update();
    }
  }

  removed() {
    this.running?.off('modified', this.switchClusterList);
    Callback.remove('ProcessOverview_setRunning');
  }

  toggleView(e) {
    this.activeTab = e.target.closest('button').id;
    localStorage.setItem('ProcessOverview_activeTab', this.activeTab);
    this.update();
  }

  render() {
    return html`
      <div class="mb-2 ms-3 d-flex justify-content-between">
        <ul class="nav nav-underline">
          <li class="nav-item">
            <button id="processes" @click="${(e) => this.toggleView(e)}" class="nav-link ${this.activeTab === 'processes' ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-2" about="v-bpa:ShowProcesses" property="rdfs:label"></span>
              <span class="align-top badge rounded-pill bg-secondary">${this.processesCount}</span>
            </button>
          </li>
          <li class="nav-item">
            <button id="documents" @click="${(e) => this.toggleView(e)}" class="nav-link ${this.activeTab === 'documents' ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-2" about="v-bpa:ProcessDocuments" property="rdfs:label"></span>
              <span class="align-top badge rounded-pill bg-secondary">${this.documentsCount}</span>
            </button>
          </li>
          <li class="nav-item">
            <button id="clusters" @click="${(e) => this.toggleView(e)}" class="nav-link ${this.activeTab === 'clusters' ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-2" about="v-bpa:ShowClusters" property="rdfs:label"></span>
              <span class="align-top badge rounded-pill bg-secondary">${this.completed?.['v-bpa:foundClusters']?.length ?? 0}</span>
            </button>
          </li>
        </ul>
        ${this.activeTab === 'clusters'
          ? html`<${ClusterizationButton} ${this.running ? `about="${this.running.id}"` : ''} callback="${Callback.getName(this.setRunning)}" class="ms-auto"></${ClusterizationButton}>`
          : this.activeTab === 'processes'
          ? html`
            <a href="#process-quick-create-modal" data-bs-toggle="modal" data-bs-target="#process-quick-create-modal" class="btn btn-link text-dark text-decoration-none ms-auto me-3">
              <i class="bi bi-plus me-1"></i>
              <span about="v-bpa:AddProcess" property="rdfs:label"></span>
            </a>
            <div class="modal fade" id="process-quick-create-modal">
              <div class="modal-dialog modal-dialog-centered">
                <div class="modal-content">
                  <div class="modal-body">
                    <${ProcessQuickCreate}></${ProcessQuickCreate}>
                  </div>
                </div>
              </div>
            </div>
            `
          : html`
            <a href="#/DocumentEdit" class="btn btn-link text-dark text-decoration-none ms-auto me-3">
              <i class="bi bi-plus me-1"></i>
              <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
            </a>`
        }
      </div>
      ${this.activeTab === 'clusters'
        ? html`<${ClusterList} ${this.completed ? `about="${this.completed.id}"` : ''}></${ClusterList}>`
        : this.activeTab === 'processes' 
        ? html`<${ProcessList}></${ProcessList}>`
        : html`<${DocumentList}></${DocumentList}>`
      }
    `;
  }
}

customElements.define(ProcessOverview.tag, ProcessOverview);

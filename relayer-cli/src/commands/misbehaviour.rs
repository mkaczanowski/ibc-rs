use abscissa_core::{config, error::BoxError, Command, Options, Runnable};
use ibc::events::IbcEvent;
use ibc::ics02_client::events::UpdateClient;
use ibc::ics02_client::height::Height;
use ibc::ics24_host::identifier::{ChainId, ClientId};
use ibc_proto::ibc::core::client::v1::QueryClientStatesRequest;
use ibc_relayer::chain::handle::ChainHandle;
use ibc_relayer::foreign_client::ForeignClient;

use crate::application::CliApp;
use crate::cli_utils::spawn_chain_runtime;
use crate::conclude::Output;
use crate::prelude::*;
use ibc::ics02_client::client_state::ClientState;

#[derive(Clone, Command, Debug, Options)]
pub struct MisbehaviourCmd {
    #[options(
        free,
        required,
        help = "identifier of the chain where client updates are monitored for misbehaviour"
    )]
    chain_id: ChainId,

    #[options(help = "identifier of the client to be monitored for misbehaviour")]
    client_id: Option<ClientId>,
}

impl Runnable for MisbehaviourCmd {
    fn run(&self) {
        let config = app_config();

        let res = monitor_misbehaviour(&self.chain_id, &self.client_id, &config);
        match res {
            Ok(()) => Output::success(()).exit(),
            Err(e) => Output::error(format!("{}", e)).exit(),
        }
    }
}

pub fn monitor_misbehaviour(
    chain_id: &ChainId,
    client_id: &Option<ClientId>,
    config: &config::Reader<CliApp>,
) -> Result<(), BoxError> {
    let chain = spawn_chain_runtime(&config, chain_id)
        .map_err(|e| format!("could not spawn the chain runtime for {}", chain_id))?;

    let subscription = chain.subscribe()?;

    // check the current states for all clients on chain
    let clients = chain
        .query_clients(QueryClientStatesRequest { pagination: None })
        .map_err(|e| format!("could not query clients for {}", chain.id()))?;

    // check previous updates that may have been missed
    match client_id {
        Some(client_id) => misbehaviour_handling(chain.clone(), config, client_id, None)?,
        None => {
            for client_id in clients.iter() {
                misbehaviour_handling(chain.clone(), config, client_id, None)?;
            }
        }
    }

    // process update client events
    while let Ok(event_batch) = subscription.recv() {
        for event in event_batch.events.iter() {
            match event {
                IbcEvent::UpdateClient(update) => {
                    if let Some(specified_client) = client_id {
                        if update.client_id() != specified_client {
                            continue;
                        }
                    }
                    dbg!(update);

                    misbehaviour_handling(
                        chain.clone(),
                        config,
                        update.client_id(),
                        Some(update.clone()),
                    )?;
                }

                IbcEvent::CreateClient(create) => {
                    // TODO - get header from full node, consensus state from chain, compare
                }

                IbcEvent::ClientMisbehaviour(misbehaviour) => {
                    // TODO - submit misbehaviour to the witnesses (our full node)
                }

                _ => {}
            }
        }
    }

    Ok(())
}

fn misbehaviour_handling(
    chain: Box<dyn ChainHandle>,
    config: &config::Reader<CliApp>,
    client_id: &ClientId,
    update: Option<UpdateClient>,
) -> Result<(), BoxError> {
    let client_state = chain
        .query_client_state(client_id, Height::zero())
        .map_err(|e| format!("could not query client state for {}", client_id))?;

    if client_state.is_frozen() {
        // nothing to do
        return Ok(());
    }
    let counterparty_chain =
        spawn_chain_runtime(&config, &client_state.chain_id()).map_err(|e| {
            format!(
                "could not spawn the chain runtime for {}",
                client_state.chain_id()
            )
        })?;

    let client =
        ForeignClient::restore_client(chain.clone(), counterparty_chain.clone(), client_id);

    let misbehaviour_detection_result = client
        .detect_misbehaviour_and_send_evidence(update)
        .map_err(|e| {
            format!(
                "could not run misbehaviour detection for {}: {}",
                client_id, e
            )
        })?;

    if let Some(evidence_submission_result) = misbehaviour_detection_result {
        info!(
            "\nEvidence submission result {:?}",
            evidence_submission_result
        );
    }

    Ok(())
}
use datasize::DataSize;
use prometheus::{self, Histogram, HistogramOpts, IntGauge, Registry};
use tracing::debug;

use super::MainReactor;
use crate::unregister_metric;

/// Metrics for estimated heap memory usage for the main reactor.
#[derive(Debug)]
pub(super) struct MemoryMetrics {
    mem_total: IntGauge,
    mem_metrics: IntGauge,
    mem_net: IntGauge,
    mem_address_gossiper: IntGauge,
    mem_storage: IntGauge,
    mem_contract_runtime: IntGauge,
    mem_rpc_server: IntGauge,
    mem_rest_server: IntGauge,
    mem_event_stream_server: IntGauge,
    mem_consensus: IntGauge,
    mem_transaction_gossiper: IntGauge,
    mem_finality_signature_gossiper: IntGauge,
    mem_block_gossiper: IntGauge,
    mem_transaction_buffer: IntGauge,
    mem_block_validator: IntGauge,
    mem_sync_leaper: IntGauge,
    mem_transaction_acceptor: IntGauge,
    mem_block_synchronizer: IntGauge,
    mem_block_accumulator: IntGauge,
    mem_fetchers: IntGauge,
    mem_diagnostics_port: IntGauge,
    mem_upgrade_watcher: IntGauge,
    mem_binary_port: IntGauge,
    /// Histogram detailing how long it took to measure memory usage.
    mem_estimator_runtime_s: Histogram,
    registry: Registry,
}

impl MemoryMetrics {
    /// Initializes a new set of memory metrics.
    pub(super) fn new(registry: Registry) -> Result<Self, prometheus::Error> {
        let mem_total = IntGauge::new("mem_total", "total memory usage in bytes")?;
        let mem_metrics = IntGauge::new("mem_metrics", "metrics memory usage in bytes")?;
        let mem_net = IntGauge::new("mem_net", "network memory usage in bytes")?;
        let mem_address_gossiper = IntGauge::new(
            "mem_address_gossiper",
            "address_gossiper memory usage in bytes",
        )?;
        let mem_storage = IntGauge::new("mem_storage", "storage memory usage in bytes")?;
        let mem_contract_runtime = IntGauge::new(
            "mem_contract_runtime",
            "contract runtime memory usage in bytes",
        )?;
        let mem_rpc_server = IntGauge::new("mem_rpc_server", "rpc server memory usage in bytes")?;
        let mem_rest_server =
            IntGauge::new("mem_rest_server", "rest server memory usage in bytes")?;
        let mem_event_stream_server = IntGauge::new(
            "mem_event_stream_server",
            "event stream server memory usage in bytes",
        )?;
        let mem_consensus = IntGauge::new("mem_consensus", "consensus memory usage in bytes")?;
        let mem_fetchers = IntGauge::new("mem_fetchers", "combined fetcher memory usage in bytes")?;
        let mem_transaction_gossiper = IntGauge::new(
            "mem_transaction_gossiper",
            "transaction gossiper memory usage in bytes",
        )?;
        let mem_finality_signature_gossiper = IntGauge::new(
            "mem_finality_signature_gossiper",
            "finality signature gossiper memory usage in bytes",
        )?;
        let mem_block_gossiper =
            IntGauge::new("mem_block_gossiper", "block gossiper memory usage in bytes")?;
        let mem_transaction_buffer = IntGauge::new(
            "mem_transaction_buffer",
            "transaction buffer memory usage in bytes",
        )?;
        let mem_block_validator = IntGauge::new(
            "mem_block_validator",
            "block validator memory usage in bytes",
        )?;
        let mem_sync_leaper =
            IntGauge::new("mem_sync_leaper", "sync leaper memory usage in bytes")?;
        let mem_transaction_acceptor = IntGauge::new(
            "mem_transaction_acceptor",
            "transaction acceptor memory usage in bytes",
        )?;
        let mem_block_synchronizer = IntGauge::new(
            "mem_block_synchronizer",
            "block synchronizer memory usage in bytes",
        )?;
        let mem_block_accumulator = IntGauge::new(
            "mem_block_accumulator",
            "block accumulator memory usage in bytes",
        )?;
        let mem_diagnostics_port = IntGauge::new(
            "mem_diagnostics_port",
            "diagnostics port memory usage in bytes",
        )?;
        let mem_upgrade_watcher = IntGauge::new(
            "mem_upgrade_watcher",
            "upgrade watcher memory usage in bytes",
        )?;
        let mem_binary_port =
            IntGauge::new("mem_binary_port", "binary port memory usage in bytes")?;
        let mem_estimator_runtime_s = Histogram::with_opts(
            HistogramOpts::new(
                "mem_estimator_runtime_s",
                "time in seconds to estimate memory usage",
            )
            //  Create buckets from one nanosecond to eight seconds.
            .buckets(prometheus::exponential_buckets(0.000_4, 2.0, 13)?),
        )?;

        registry.register(Box::new(mem_total.clone()))?;
        registry.register(Box::new(mem_metrics.clone()))?;
        registry.register(Box::new(mem_net.clone()))?;
        registry.register(Box::new(mem_address_gossiper.clone()))?;
        registry.register(Box::new(mem_storage.clone()))?;
        registry.register(Box::new(mem_contract_runtime.clone()))?;
        registry.register(Box::new(mem_rpc_server.clone()))?;
        registry.register(Box::new(mem_rest_server.clone()))?;
        registry.register(Box::new(mem_event_stream_server.clone()))?;
        registry.register(Box::new(mem_consensus.clone()))?;
        registry.register(Box::new(mem_fetchers.clone()))?;
        registry.register(Box::new(mem_transaction_gossiper.clone()))?;
        registry.register(Box::new(mem_finality_signature_gossiper.clone()))?;
        registry.register(Box::new(mem_block_gossiper.clone()))?;
        registry.register(Box::new(mem_transaction_buffer.clone()))?;
        registry.register(Box::new(mem_block_validator.clone()))?;
        registry.register(Box::new(mem_sync_leaper.clone()))?;
        registry.register(Box::new(mem_transaction_acceptor.clone()))?;
        registry.register(Box::new(mem_block_synchronizer.clone()))?;
        registry.register(Box::new(mem_block_accumulator.clone()))?;
        registry.register(Box::new(mem_diagnostics_port.clone()))?;
        registry.register(Box::new(mem_upgrade_watcher.clone()))?;
        registry.register(Box::new(mem_binary_port.clone()))?;
        registry.register(Box::new(mem_estimator_runtime_s.clone()))?;

        Ok(MemoryMetrics {
            mem_total,
            mem_metrics,
            mem_net,
            mem_address_gossiper,
            mem_storage,
            mem_contract_runtime,
            mem_rpc_server,
            mem_rest_server,
            mem_event_stream_server,
            mem_consensus,
            mem_fetchers,
            mem_transaction_gossiper,
            mem_finality_signature_gossiper,
            mem_block_gossiper,
            mem_transaction_buffer,
            mem_block_validator,
            mem_sync_leaper,
            mem_transaction_acceptor,
            mem_block_synchronizer,
            mem_block_accumulator,
            mem_diagnostics_port,
            mem_upgrade_watcher,
            mem_binary_port,
            mem_estimator_runtime_s,
            registry,
        })
    }

    /// Estimates memory usage and updates metrics.
    pub(super) fn estimate(&self, reactor: &MainReactor) {
        let timer = self.mem_estimator_runtime_s.start_timer();

        let metrics = reactor.metrics.estimate_heap_size() as i64;
        let network = reactor.net.estimate_heap_size() as i64;
        let address_gossiper = reactor.address_gossiper.estimate_heap_size() as i64;
        let storage = reactor.storage.estimate_heap_size() as i64;
        let contract_runtime = reactor.contract_runtime.estimate_heap_size() as i64;
        let rest_server = reactor.rest_server.estimate_heap_size() as i64;
        let event_stream_server = reactor.event_stream_server.estimate_heap_size() as i64;
        let consensus = reactor.consensus.estimate_heap_size() as i64;
        let fetchers = reactor.fetchers.estimate_heap_size() as i64;
        let transaction_gossiper = reactor.transaction_gossiper.estimate_heap_size() as i64;
        let finality_signature_gossiper =
            reactor.finality_signature_gossiper.estimate_heap_size() as i64;
        let block_gossiper = reactor.block_gossiper.estimate_heap_size() as i64;
        let transaction_buffer = reactor.transaction_buffer.estimate_heap_size() as i64;
        let block_validator = reactor.block_validator.estimate_heap_size() as i64;
        let sync_leaper = reactor.sync_leaper.estimate_heap_size() as i64;
        let transaction_acceptor = reactor.transaction_acceptor.estimate_heap_size() as i64;
        let block_synchronizer = reactor.block_synchronizer.estimate_heap_size() as i64;
        let block_accumulator = reactor.block_accumulator.estimate_heap_size() as i64;
        let diagnostics_port = reactor.diagnostics_port.estimate_heap_size() as i64;
        let upgrade_watcher = reactor.upgrade_watcher.estimate_heap_size() as i64;
        let binary_port = reactor.binary_port.estimate_heap_size() as i64;

        let total = metrics
            + network
            + address_gossiper
            + storage
            + contract_runtime
            + rest_server
            + event_stream_server
            + consensus
            + fetchers
            + transaction_gossiper
            + finality_signature_gossiper
            + block_gossiper
            + transaction_buffer
            + block_validator
            + sync_leaper
            + transaction_acceptor
            + block_synchronizer
            + block_accumulator
            + diagnostics_port
            + upgrade_watcher
            + binary_port;

        self.mem_net.set(network);
        self.mem_address_gossiper.set(address_gossiper);
        self.mem_storage.set(storage);
        self.mem_contract_runtime.set(contract_runtime);
        self.mem_rest_server.set(rest_server);
        self.mem_event_stream_server.set(event_stream_server);
        self.mem_consensus.set(consensus);
        self.mem_fetchers.set(fetchers);
        self.mem_transaction_gossiper.set(transaction_gossiper);
        self.mem_finality_signature_gossiper
            .set(finality_signature_gossiper);
        self.mem_block_gossiper.set(block_gossiper);
        self.mem_transaction_buffer.set(transaction_buffer);
        self.mem_block_validator.set(block_validator);
        self.mem_sync_leaper.set(sync_leaper);
        self.mem_transaction_acceptor.set(transaction_acceptor);
        self.mem_block_synchronizer.set(block_synchronizer);
        self.mem_block_accumulator.set(block_accumulator);
        self.mem_diagnostics_port.set(diagnostics_port);
        self.mem_upgrade_watcher.set(upgrade_watcher);
        self.mem_binary_port.set(binary_port);

        self.mem_total.set(total);
        self.mem_metrics.set(metrics);

        // Stop the timer explicitly, don't count logging.
        let duration_s = timer.stop_and_record();

        debug!(%total,
               %duration_s,
               %metrics,
               %network,
               %address_gossiper,
               %storage,
               %contract_runtime,
               %rest_server,
               %event_stream_server,
               %consensus,
               %fetchers,
               %transaction_gossiper,
               %finality_signature_gossiper,
               %block_gossiper,
               %transaction_buffer,
               %block_validator,
               %sync_leaper,
               %transaction_acceptor,
               %block_synchronizer,
               %block_accumulator,
               %diagnostics_port,
               %upgrade_watcher,
               %binary_port,
               "Collected new set of memory metrics.");
    }
}

impl Drop for MemoryMetrics {
    fn drop(&mut self) {
        unregister_metric!(self.registry, self.mem_total);
        unregister_metric!(self.registry, self.mem_metrics);
        unregister_metric!(self.registry, self.mem_estimator_runtime_s);

        unregister_metric!(self.registry, self.mem_net);
        unregister_metric!(self.registry, self.mem_address_gossiper);
        unregister_metric!(self.registry, self.mem_storage);
        unregister_metric!(self.registry, self.mem_contract_runtime);
        unregister_metric!(self.registry, self.mem_rpc_server);
        unregister_metric!(self.registry, self.mem_rest_server);
        unregister_metric!(self.registry, self.mem_event_stream_server);
        unregister_metric!(self.registry, self.mem_consensus);
        unregister_metric!(self.registry, self.mem_fetchers);
        unregister_metric!(self.registry, self.mem_transaction_gossiper);
        unregister_metric!(self.registry, self.mem_finality_signature_gossiper);
        unregister_metric!(self.registry, self.mem_block_gossiper);
        unregister_metric!(self.registry, self.mem_transaction_buffer);
        unregister_metric!(self.registry, self.mem_block_validator);
        unregister_metric!(self.registry, self.mem_sync_leaper);
        unregister_metric!(self.registry, self.mem_transaction_acceptor);
        unregister_metric!(self.registry, self.mem_block_synchronizer);
        unregister_metric!(self.registry, self.mem_block_accumulator);
        unregister_metric!(self.registry, self.mem_diagnostics_port);
        unregister_metric!(self.registry, self.mem_upgrade_watcher);
        unregister_metric!(self.registry, self.mem_binary_port);
    }
}

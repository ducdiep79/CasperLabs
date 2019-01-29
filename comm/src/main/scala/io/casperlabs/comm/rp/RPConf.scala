package io.casperlabs.comm.rp

import scala.concurrent.duration._

import io.casperlabs.comm.PeerNode

final case class RPConf(
    local: PeerNode,
    bootstrap: Option[PeerNode],
    defaultTimeout: FiniteDuration,
    clearConnections: ClearConnetionsConf
)
final case class ClearConnetionsConf(maxNumOfConnections: Int, numOfConnectionsPinged: Int)

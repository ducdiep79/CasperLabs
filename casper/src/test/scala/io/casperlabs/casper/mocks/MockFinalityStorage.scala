package io.casperlabs.casper.mocks

import cats._
import cats.implicits._
import cats.effect._
import cats.effect.concurrent.Ref
import io.casperlabs.storage.BlockHash
import io.casperlabs.storage.dag.FinalityStorage

class MockFinalityStorage[F[_]: Monad](
    lastFinalizedRef: Ref[F, BlockHash],
    finalizedRef: Ref[F, Set[BlockHash]]
) extends FinalityStorage[F] {
  override def getLastFinalizedBlock: F[BlockHash] =
    lastFinalizedRef.get
  override def isFinalized(block: BlockHash): F[Boolean] =
    finalizedRef.get.map(_ contains block)
  override def markAsFinalized(mainParent: BlockHash, secondary: Set[BlockHash]): F[Unit] =
    lastFinalizedRef.set(mainParent) >> finalizedRef.update {
      _ ++ secondary + mainParent
    }
}

object MockFinalityStorage {
  def apply[F[_]: Sync](blocks: BlockHash*): F[MockFinalityStorage[F]] =
    for {
      lastFinalizedRef <- Ref.of[F, BlockHash](blocks.last)
      finalizedRef     <- Ref.of[F, Set[BlockHash]](blocks.toSet)
    } yield new MockFinalityStorage(lastFinalizedRef, finalizedRef)
}
